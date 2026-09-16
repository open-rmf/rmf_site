use crate::event::{CandidateDiscreteEvents, DynDiscreteEvent};
use crate::schedule::{SimulationPredict, SimulationStartup};
use crate::simulation::{
    SimulationComputeState, SimulationComputeUpdate, SimulationPluginFactory, SimulationState,
    SimulationStep,
};
use crate::sync::{StateUpdateSender, Synchronizer};
use crate::time::SimulationClock;
use bevy::prelude::*;
use bevy::tasks::{AsyncComputeTaskPool, TaskPool};
use crossbeam_channel::Sender;
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::time::{Duration, Instant};

/// Settings for the computation of a simulation.
#[derive(Clone, Copy, Debug)]
pub struct SimulationComputeSettings {
    /// The maximum number of instantaneous events that may be executed for a single simulation time step.
    ///
    /// Exceeding this limit panics, terminating the computation with
    /// [`SimulationComputeState::Failed`].
    pub max_events_per_step: Option<u64>,
}

impl Default for SimulationComputeSettings {
    fn default() -> Self {
        Self {
            max_events_per_step: Some(u64::MAX),
        }
    }
}

/// Computes [`SimulationStep`]s for a simulation in a [`AsyncComputeTaskPool`].
pub fn compute_async(
    init_state: SimulationState,
    startup_schedule: Schedule,
    prediction_schedule: Schedule,
    plugins: Vec<SimulationPluginFactory>,
    settings: SimulationComputeSettings,
    synchronizer: Synchronizer,
    sender: Sender<SimulationComputeUpdate>,
) {
    let completion_sender = sender.clone();
    AsyncComputeTaskPool::get_or_init(TaskPool::new)
        .spawn(async move {
            let result = catch_unwind(AssertUnwindSafe(move || {
                let mut app = build_app(
                    init_state,
                    startup_schedule,
                    prediction_schedule,
                    plugins,
                    settings,
                    synchronizer,
                    sender,
                );
                app.run();
            }));
            let state = match result {
                Ok(()) => SimulationComputeState::Complete,
                Err(_) => SimulationComputeState::Failed,
            };
            let _ = completion_sender.send(SimulationComputeUpdate::State(state));
        })
        .detach();
}

/// Builds an [`App`] with the initial state, plugins, schedules, and runner.
fn build_app(
    init_state: SimulationState,
    startup_schedule: Schedule,
    prediction_schedule: Schedule,
    plugins: Vec<SimulationPluginFactory>,
    settings: SimulationComputeSettings,
    synchronizer: Synchronizer,
    sender: Sender<SimulationComputeUpdate>,
) -> App {
    let mut app = App::new();
    app.add_schedule(startup_schedule);
    app.add_schedule(prediction_schedule);
    app.init_resource::<SimulationClock>()
        .init_resource::<CandidateDiscreteEvents>()
        .insert_resource(StateUpdateSender::new(sender));
    app.set_runner(move |app| runner(app, settings));

    for apply_plugin in plugins {
        apply_plugin(&mut app);
    }

    synchronizer.sync(&init_state.0, app.world_mut());
    app
}

/// A runner to compute and send one [`crate::simulation::SimulationStep`] at a time.
fn runner(mut app: App, settings: SimulationComputeSettings) -> AppExit {
    let world = app.world_mut();
    world.run_schedule(SimulationStartup);

    // Seed initial candidate events, and advance clock to the first time step.
    world.run_schedule(SimulationPredict);
    advance_clock_to_next_event(world);

    while let Some(step) = compute_step(world, settings.max_events_per_step) {
        send_step(world, step);
        advance_clock_to_next_event(world);
    }

    // TODO(@reuben-thomas):
    // Configurable end conditions could include:
    // - A maximum number of steps:
    // - A generic user implemented trait e.g. `crate::simulation::EndCondition`
    // - A user system that sends an event
    info!(
        "Finished computing simulation, no new events were produced at time {:?}",
        world.resource::<SimulationClock>().now()
    );
    AppExit::Success
}

/// Advances the clock to the time of the next candidate event, if any.
fn advance_clock_to_next_event(world: &mut World) {
    let Some(next_time) = world.resource::<CandidateDiscreteEvents>().next_time() else {
        return;
    };
    world
        .resource_mut::<SimulationClock>()
        .advance_to(next_time);
}

/// Computes a single simulation step at the current simulation time.
///
/// Returns [`None`] once no event is due, which is how the simulation ends.
fn compute_step(world: &mut World, max_events: Option<u64>) -> Option<SimulationStep> {
    let mut events = Vec::new();

    while let Some(event) = execute_highest_priority_current_event(world) {
        events.push(event);
        world.run_schedule(SimulationPredict);

        if let Some(max_events) = max_events
            && events.len() as u64 >= max_events
        {
            panic!(
                "Reached the maximum of {max_events} instantaneous events at time {:?}",
                world.resource::<SimulationClock>().now()
            );
        }
    }

    if events.is_empty() {
        None
    } else {
        Some(SimulationStep::new(events))
    }
}

/// Executes the highest priority candidate event due at the current simulation
/// time if any, and then discards all other candidate events.
fn execute_highest_priority_current_event(world: &mut World) -> Option<Box<dyn DynDiscreteEvent>> {
    let now = world.resource::<SimulationClock>().now();

    let mut candidates = world.resource_mut::<CandidateDiscreteEvents>();
    if candidates.next_time() != Some(now) {
        return None;
    }
    let event = candidates.pop_highest_priority_event()?;
    candidates.discard_all();

    event.clone().apply(world);
    Some(event)
}

/// Sends a computed step to the main app at the current simulation time.
fn send_step(world: &mut World, step: SimulationStep) {
    let now = world.resource::<SimulationClock>().now();
    world.resource::<StateUpdateSender>().send(now, step);
}

// TODO(@reuben-thomas)
/// A timer to measure the time taken by a simulation compute task.
#[derive(Clone, Copy, Debug)]
pub struct SimulationComputeTimer {
    started: Instant,
    total: Option<Duration>,
}

impl Default for SimulationComputeTimer {
    fn default() -> Self {
        Self::start()
    }
}

impl SimulationComputeTimer {
    /// Creates a timer that begins measuring time immediately.
    pub fn start() -> Self {
        Self {
            started: Instant::now(),
            total: None,
        }
    }

    /// Stops the timer.
    pub fn stop(&mut self) {
        self.total = Some(self.started.elapsed());
    }

    /// The elapsed time.
    pub fn elapsed(&self) -> Duration {
        self.total.unwrap_or_else(|| self.started.elapsed())
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::CandidateEventWriter;
    use crate::time::SimulationTime;
    use bevy::ecs::system::Command;
    use std::time::Duration;

    const EXPECTED_FINAL_COUNT: u32 = 10;

    /// The number of times [`Increment`] was executed.
    #[derive(Resource, Default)]
    struct Count(u32);

    #[derive(Clone, Debug)]
    struct IncrementCount;

    impl Command for IncrementCount {
        fn apply(self, world: &mut World) {
            world.resource_mut::<Count>().0 += 1;
        }
    }

    /// Predicts an [`IncrementCount`] once every second in the future until [`EXPECTED_FINAL_COUNT`] is reached.
    fn increment(clock: Res<SimulationClock>, count: Res<Count>, mut writer: CandidateEventWriter) {
        if count.0 < EXPECTED_FINAL_COUNT {
            writer.predict(clock.now() + Duration::from_secs(1), IncrementCount);
        }
    }

    #[test]
    fn test_compute_simulation() {
        let (sender, receiver) = crossbeam_channel::unbounded();

        let mut prediction_schedule = Schedule::new(SimulationPredict);
        prediction_schedule.add_systems(increment);

        let mut app = App::new();
        app.add_schedule(Schedule::new(SimulationStartup));
        app.add_schedule(prediction_schedule);
        app.init_resource::<SimulationClock>()
            .init_resource::<CandidateDiscreteEvents>()
            .init_resource::<Count>()
            .insert_resource(StateUpdateSender::new(sender));
        app.set_runner(|app| runner(app, SimulationComputeSettings::default()));
        app.run();

        let expected_time_to_event_count = (1..=EXPECTED_FINAL_COUNT as u64)
            .map(|second| (SimulationTime::new(Duration::from_secs(second)), 1))
            .collect::<Vec<_>>();
        let actual_time_to_event_count: Vec<_> = receiver
            .try_iter()
            .filter_map(|update| match update {
                SimulationComputeUpdate::Step(time, step) => Some((time, step.event_count())),
                SimulationComputeUpdate::State(_) => None,
            })
            .collect();

        assert_eq!(actual_time_to_event_count, expected_time_to_event_count,);
    }
}
