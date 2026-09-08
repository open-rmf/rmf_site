use bevy::ecs::schedule::{
    InternedScheduleLabel, IntoScheduleConfigs, LogLevel, NodeId, Schedule, ScheduleBuildError,
    ScheduleBuildPass, ScheduleBuildSettings, ScheduleConfigs, ScheduleGraph, ScheduleLabel,
    SystemSet,
    graph::{DiGraph, Direction},
};
use bevy::ecs::system::ScheduleSystem;
use bevy::ecs::world::World;
use std::any::TypeId;
use std::collections::{BTreeSet, HashMap, HashSet};

/// The schedule that runs once when the a simulation is started.
#[derive(Clone, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct SimulationStartup;

/// The schedule executing prediction systems that predict events based on the
/// current state of the world.
#[derive(Clone, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct SimulationPredict;

/// The schedule run in the main world to visualize a simulation while it is the
/// active playback simulation.
#[derive(Clone, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
pub struct SimulationVisualize;

#[derive(Default, Clone)]
pub enum SystemExecutionOrdering {
    /// Systems may run in any order based on dependencies and user-defined
    /// ordering constraints.
    #[default]
    Partial,
    /// A deterministic total order is enforced.
    Total,
}

/// Builds a [`Schedule`] with a set of systems and an execution ordering
/// policy.
pub struct ScheduleBuilder {
    label: InternedScheduleLabel,
    configs: Option<ScheduleConfigs<ScheduleSystem>>,
    ordering: SystemExecutionOrdering,
}

impl ScheduleBuilder {
    /// Creates a new empty builder for a schedule.
    pub fn new(label: impl ScheduleLabel) -> Self {
        Self {
            label: label.intern(),
            configs: None,
            ordering: SystemExecutionOrdering::default(),
        }
    }

    /// Adds systems to the schedule.
    pub fn add_systems<M>(mut self, systems: impl IntoScheduleConfigs<ScheduleSystem, M>) -> Self {
        let configs = systems.into_configs();
        self.configs = Some(match self.configs.take() {
            Some(existing) => (existing, configs).into_configs(),
            None => configs,
        });
        self
    }

    /// Adds systems to the schedule for a system set.
    pub fn add_systems_in_set<M>(
        self,
        systems: impl IntoScheduleConfigs<ScheduleSystem, M>,
        set: impl SystemSet,
    ) -> Self {
        self.add_systems(systems.in_set(set))
    }

    /// Sets the execution ordering policy for the schedule.
    pub fn set_ordering(mut self, ordering: SystemExecutionOrdering) -> Self {
        self.ordering = ordering;
        self
    }

    /// Builds a new [`Schedule`] containing the added systems, ordered by the
    /// configured policy.
    pub fn build(self) -> Schedule {
        let mut schedule = Schedule::new(self.label);
        if let Some(configs) = self.configs {
            schedule.add_systems(configs);
        }

        match self.ordering {
            SystemExecutionOrdering::Total => {
                schedule.set_build_settings(ScheduleBuildSettings {
                    ambiguity_detection: LogLevel::Error,
                    ..Default::default()
                });
                schedule.add_build_pass(TotalOrderingBuildPass {
                    added_system_type_ids: schedule
                        .graph()
                        .systems()
                        .map(|(_, system, _)| system.type_id())
                        .collect(),
                });
            }
            SystemExecutionOrdering::Partial => {}
        }
        schedule
    }
}

/// A schedule build pass that enforces a deterministic total ordering of
/// systems.
#[derive(Debug, Default)]
struct TotalOrderingBuildPass {
    added_system_type_ids: HashSet<TypeId>,
}

impl ScheduleBuildPass for TotalOrderingBuildPass {
    type EdgeOptions = ();

    fn add_dependency(&mut self, _from: NodeId, _to: NodeId, _options: Option<&Self::EdgeOptions>) {
    }

    fn collapse_set(
        &mut self,
        _set: NodeId,
        _systems: &[NodeId],
        _dependency_flattened: &DiGraph,
    ) -> impl Iterator<Item = (NodeId, NodeId)> {
        std::iter::empty()
    }

    // TODO(@reuben-thomas): Toposort for each ScheduleGraph::conflicting_systems
    // set
    fn build(
        &mut self,
        _world: &mut World,
        graph: &mut ScheduleGraph,
        dependency_flattened: &mut DiGraph,
    ) -> Result<(), ScheduleBuildError> {
        let mut node_to_in_degree: HashMap<NodeId, usize> =
            dependency_flattened.nodes().map(|node| (node, 0)).collect();
        for (_, to) in dependency_flattened.all_edges() {
            *node_to_in_degree.get_mut(&to).unwrap() += 1;
        }

        let mut topological_order: Vec<NodeId> = Vec::with_capacity(node_to_in_degree.len());
        let mut node_queue: BTreeSet<(String, NodeId)> = node_to_in_degree
            .iter()
            .filter(|(_, degree)| **degree == 0)
            .map(|(node, _)| (TotalOrderingBuildPass::get_system_name(graph, node), *node))
            .collect();

        while let Some((name, node)) = node_queue.pop_first() {
            if self.process_system(graph, &node) {
                println!("Processing {name}...");
                topological_order.push(node);
            }

            for outgoing_neighbor in
                dependency_flattened.neighbors_directed(node, Direction::Outgoing)
            {
                let degree = node_to_in_degree.get_mut(&outgoing_neighbor).unwrap();
                *degree -= 1;
                if *degree == 0 {
                    node_queue.insert((
                        TotalOrderingBuildPass::get_system_name(graph, &outgoing_neighbor),
                        outgoing_neighbor,
                    ));
                }
            }
        }

        for pair in topological_order.windows(2) {
            dependency_flattened.add_edge(pair[0], pair[1]);
        }

        Ok(())
    }
}

impl TotalOrderingBuildPass {
    fn process_system(&self, graph: &ScheduleGraph, id: &NodeId) -> bool {
        graph
            .get_system_at(*id)
            .is_some_and(|system| self.added_system_type_ids.contains(&system.type_id()))
    }

    fn get_system_name(graph: &ScheduleGraph, id: &NodeId) -> String {
        graph
            .get_system_at(*id)
            .map(|system| system.name().to_string())
            .unwrap_or_default()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::event::CandidateEventWriter;
    use bevy::ecs::resource::Resource;
    use bevy::ecs::system::{IntoSystem, ResMut, System};

    #[derive(Clone, Debug, PartialEq, Eq, Hash, ScheduleLabel)]
    struct TestSchedule;

    #[derive(Resource, Default, Clone)]
    struct X<const N: usize>;

    fn a(_x1: ResMut<X<1>>, _writer: CandidateEventWriter) {}

    fn b(_x2: ResMut<X<2>>, _writer: CandidateEventWriter) {}

    fn c(_x1: ResMut<X<1>>, _x2: ResMut<X<2>>, _writer: CandidateEventWriter) {}

    fn get_system_name<M>(system: impl IntoSystem<(), (), M>) -> String {
        IntoSystem::into_system(system).name().to_string()
    }

    #[test]
    fn test_total_ordering() {
        let mut world = World::new();
        let mut schedule = ScheduleBuilder::new(TestSchedule)
            .add_systems((a, b.before(a), c.after(b)))
            .set_ordering(SystemExecutionOrdering::Total)
            .build();
        schedule.initialize(&mut world).unwrap();

        let expected = vec![get_system_name(b), get_system_name(a), get_system_name(c)];
        let actual: Vec<_> = schedule
            .systems()
            .unwrap()
            .map(|(_, system)| system.name().to_string())
            .filter(|name| expected.contains(name))
            .collect();
        assert_eq!(actual, expected);
    }

    #[test]
    fn test_ordering_ambiguity_enforced() {
        let mut world = World::new();

        let mut schedule = ScheduleBuilder::new(TestSchedule)
            .add_systems(a)
            .set_ordering(SystemExecutionOrdering::Partial)
            .build();
        schedule.add_systems(c);
        assert!(schedule.initialize(&mut world).is_ok());

        let mut schedule = ScheduleBuilder::new(TestSchedule)
            .add_systems(a)
            .set_ordering(SystemExecutionOrdering::Total)
            .build();
        schedule.add_systems(c);
        let result = schedule.initialize(&mut world);
        assert!(matches!(result, Err(ScheduleBuildError::Ambiguity(_))));
    }
}
