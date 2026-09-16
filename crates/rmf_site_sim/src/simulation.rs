use crate::compute::{SimulationComputeSettings, SimulationComputeTimer, compute_async};
use crate::event::DynDiscreteEvent;
use crate::schedule::{
    ScheduleBuilder, SimulationPredict, SimulationStartup, SimulationVisualize,
    SystemExecutionOrdering,
};
use crate::sync::Synchronizer;
use crate::time::SimulationTime;
use bevy::app::Plugins;
use bevy::ecs::system::ScheduleSystem;
use bevy::prelude::*;
use crossbeam_channel::{Receiver, unbounded};
use std::collections::BTreeMap;
use std::marker::PhantomData;
use std::time::Duration;

/// Plugin for computing discrete event simulations.
pub struct SimulationPlugin;

impl Plugin for SimulationPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, Simulation::process_updates);
    }
}

/// Adds one or more plugins to the target world's [`App`].
pub type SimulationPluginFactory = Box<dyn FnOnce(&mut App) + Send>;

/// Builds a [`Simulation`].
pub struct SimulationBuilder<M: Component + Clone> {
    synchronizer: Synchronizer,
    marker: PhantomData<M>,
    startup_schedule_builder: ScheduleBuilder,
    prediction_schedule_builder: ScheduleBuilder,
    visualization_schedule_builder: ScheduleBuilder,
    plugins: Vec<SimulationPluginFactory>,
    compute_settings: SimulationComputeSettings,
}

impl<M: Component + Clone> Default for SimulationBuilder<M> {
    fn default() -> Self {
        Self::new()
    }
}

/// Builder for creating a [`Simulation`].
///
/// All registered resources and components, as well as their corresponding
/// entities with the marker component `M` are extracted into the simulation
/// world.
impl<M: Component + Clone> SimulationBuilder<M> {
    pub fn new() -> Self {
        let mut synchronizer = Synchronizer::new();
        synchronizer.register_component::<M>();
        Self {
            synchronizer,
            startup_schedule_builder: ScheduleBuilder::new(SimulationStartup),
            prediction_schedule_builder: ScheduleBuilder::new(SimulationPredict)
                .set_ordering(SystemExecutionOrdering::Total),
            visualization_schedule_builder: ScheduleBuilder::new(SimulationVisualize),
            plugins: Vec::new(),
            compute_settings: SimulationComputeSettings::default(),
            marker: PhantomData,
        }
    }

    /// Configure settings for how this simulation should be computed.
    pub fn set_compute_settings(mut self, settings: SimulationComputeSettings) -> Self {
        self.compute_settings = settings;
        self
    }

    /// Registers a component to synchronize into the simulation world.
    pub fn register_component<T: Component + Clone>(mut self) -> Self {
        self.synchronizer.register_component::<T>();
        self
    }

    /// Registers a resource to synchronize into the simulation world.
    pub fn register_resource<T: Resource + Clone>(mut self) -> Self {
        self.synchronizer.register_resource::<T>();
        self
    }

    /// Adds a plugin to the simulation [`App`].
    pub fn add_plugins<P>(mut self, plugins: impl Plugins<P> + Send + 'static) -> Self {
        self.plugins.push(Box::new(move |app| {
            app.add_plugins(plugins);
        }));
        self
    }

    /// Adds a set of systems to be executed during simulation startup.
    pub fn add_startup_systems<S>(
        mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, S>,
    ) -> Self {
        self.startup_schedule_builder = self.startup_schedule_builder.add_systems(systems);
        self
    }

    /// Adds a set of systems to be executed when computing simulation steps.
    /// These systems should represent models in the discrete event simulation,
    /// predicting the events they believe should occur.
    pub fn add_prediction_systems<S>(
        mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, S>,
    ) -> Self {
        self.prediction_schedule_builder = self.prediction_schedule_builder.add_systems(systems);
        self
    }

    /// Adds a set of systems to be run in the main world, while this simulation
    /// is being played back.
    pub fn add_visualization_systems<S>(
        mut self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, S>,
    ) -> Self {
        self.visualization_schedule_builder =
            self.visualization_schedule_builder.add_systems(systems);
        self
    }

    /// Builds a new [`Simulation`].
    pub fn build(self, world: &World) -> Simulation {
        Simulation::new(self, world)
    }
}

/// A unique discrete event simulation.
#[derive(Component)]
pub struct Simulation {
    init_state: SimulationState,
    synchronizer: Synchronizer,
    simulation_steps: BTreeMap<SimulationTime, SimulationStep>,
    state: SimulationComputeState,
    compute_timer: SimulationComputeTimer,
    update_receiver: Receiver<SimulationComputeUpdate>,
    visualization_schedule: Schedule,
}

impl Simulation {
    // Simulation::extract, Simulation::compute
    fn new<M: Component + Clone>(builder: SimulationBuilder<M>, world: &World) -> Self {
        let (sender, receiver) = unbounded();
        let compute_timer = SimulationComputeTimer::start();
        compute_async(
            SimulationState::extract_with::<M>(&builder.synchronizer, world),
            builder.startup_schedule_builder.build(),
            builder.prediction_schedule_builder.build(),
            builder.plugins,
            builder.compute_settings,
            builder.synchronizer.clone(),
            sender,
        );

        Self {
            init_state: SimulationState::extract_with::<M>(&builder.synchronizer, world),
            synchronizer: builder.synchronizer,
            simulation_steps: BTreeMap::new(),
            state: SimulationComputeState::Computing,
            compute_timer,
            update_receiver: receiver,
            visualization_schedule: builder.visualization_schedule_builder.build(),
        }
    }

    pub fn state(&self) -> SimulationComputeState {
        self.state
    }

    pub fn compute_elapsed(&self) -> Duration {
        self.compute_timer.elapsed()
    }

    pub fn init_state(&self) -> &SimulationState {
        &self.init_state
    }

    pub fn synchronizer(&self) -> &Synchronizer {
        &self.synchronizer
    }

    // TODO(@reuben-thomas): View usage
    pub fn take_visualization_schedule(&mut self) -> Schedule {
        std::mem::replace(
            &mut self.visualization_schedule,
            Schedule::new(SimulationVisualize),
        )
    }

    pub fn restore_visualization_schedule(&mut self, schedule: Schedule) {
        self.visualization_schedule = schedule;
    }

    /// The computed simulation steps, ordered by simulation time.
    pub fn steps(&self) -> &BTreeMap<SimulationTime, SimulationStep> {
        &self.simulation_steps
    }

    /// The elapsed time of the last computed step, or zero when no steps have
    /// been computed yet.
    pub fn duration(&self) -> Duration {
        self.simulation_steps
            .keys()
            .last()
            .copied()
            .unwrap_or_default()
            .elapsed()
    }

    /// Applies [`StateUpdate`]s received from the compute task.
    fn process_updates(mut simulations: Query<&mut Simulation>) {
        for mut simulation in &mut simulations {
            let simulation = &mut *simulation;
            for update in simulation.update_receiver.try_iter() {
                match update {
                    SimulationComputeUpdate::Step(time, step) => {
                        simulation.simulation_steps.insert(time, step);
                    }
                    SimulationComputeUpdate::State(state) => {
                        simulation.state = state;
                        simulation.compute_timer.stop();
                    }
                }
            }
        }
    }
}

/// A snapshot of a world's synchronized entities, components, and resources.
pub struct SimulationState(pub World);

impl SimulationState {
    /// Extracts a state containing all entities with registered components.
    pub fn extract(synchronizer: &Synchronizer, source: &World) -> Self {
        let mut world = World::new();
        synchronizer.sync(source, &mut world);
        Self(world)
    }

    /// Extracts a state containing only entities with the marker component `M`.
    pub fn extract_with<M: Component>(synchronizer: &Synchronizer, source: &World) -> Self {
        let mut world = World::new();
        synchronizer.sync_with::<M>(source, &mut world);
        Self(world)
    }
}

/// The current state of a simulation's computation.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SimulationComputeState {
    Computing,
    Complete,
    Failed,
}

/// A single computed simulation step.
#[derive(Clone)]
pub struct SimulationStep {
    events: Vec<Box<dyn DynDiscreteEvent>>,
}

impl SimulationStep {
    /// Creates a new [`SimulationStep`] from a vector of events.
    pub fn new(events: Vec<Box<dyn DynDiscreteEvent>>) -> Self {
        Self { events }
    }

    pub fn apply(self, world: &mut World) {
        for event in self.events {
            event.apply(world);
        }
    }

    /// The number of events executed in this step.
    pub fn event_count(&self) -> usize {
        self.events.len()
    }

    /// Each event executed in this step, in execution order.
    pub fn events(&self) -> impl Iterator<Item = &dyn DynDiscreteEvent> + '_ {
        self.events.iter().map(|event| event.as_ref())
    }
}

pub enum SimulationComputeUpdate {
    Step(SimulationTime, SimulationStep),
    State(SimulationComputeState),
}
