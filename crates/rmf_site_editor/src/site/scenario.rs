/*
 * Copyright (C) 2024 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

use crate::{
    site::{
        count_scenarios_with_inclusion, Affiliation, CurrentScenario, Delete, Dependents, Element,
        GetModifier, Group, Inclusion, InstanceMarker, IssueKey, LaneMarker, LocationTags,
        ModelMarker, Modifier, NameInSite, Pending, PendingModel, ScenarioBundle,
        ScenarioModifiers, UseModifier,
    },
    CurrentWorkspace, Issue, ValidateWorkspace,
};
use bevy::ecs::hierarchy::ChildOf;
use bevy::prelude::*;
use std::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

#[derive(Clone, Debug, Event)]
pub struct CreateScenario {
    pub name: Option<String>,
    pub parent: Option<Entity>,
}

#[derive(Clone, Copy, Debug, Default, Deref, DerefMut, Resource)]
pub struct DefaultScenario(pub Option<Entity>);

impl Element for InstanceMarker {}

/// Handles updates when the current scenario has changed, and trigger property updates for scenario elements
pub fn update_current_scenario(
    mut commands: Commands,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    instances: Query<Entity, (With<InstanceMarker>, Without<PendingModel>)>,
    lanes: Query<Entity, (With<LaneMarker>, Without<Pending>)>,
    locations: Query<Entity, (With<LocationTags>, Without<Pending>)>,
) {
    if let Some(ChangeCurrentScenario(scenario_entity)) = change_current_scenario.read().last() {
        *current_scenario = CurrentScenario(Some(*scenario_entity));
        for entity in instances
            .iter()
            .chain(lanes.iter())
            .chain(locations.iter())
        {
            commands.trigger(UseModifier::new(entity, *scenario_entity));
        }
    }
}

#[derive(Clone, Debug, Event)]
pub struct ChangeDefaultScenario(pub Option<Entity>);

/// Handles updates when the default scenario has changed
pub fn update_default_scenario(
    mut change_default_scenario: EventReader<ChangeDefaultScenario>,
    mut default_scenario: ResMut<DefaultScenario>,
) {
    if let Some(ChangeDefaultScenario(optional_entity)) = change_default_scenario.read().last() {
        default_scenario.0 = *optional_entity;
    }
}

/// Create a new scenario and its children entities
pub fn handle_create_scenarios(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut new_scenarios: EventReader<CreateScenario>,
    current_workspace: Res<CurrentWorkspace>,
) {
    for new in new_scenarios.read() {
        let mut cmd = commands.spawn((ScenarioBundle::<Entity>::new(
            new.name.clone(),
            new.parent.clone(),
        ),));

        if let Some(site_entity) = current_workspace.root {
            cmd.insert(ChildOf(site_entity));
        } else {
            error!("Missing workspace for a new root scenario!");
        }
        change_current_scenario.write(ChangeCurrentScenario(cmd.id()));
    }
}

#[derive(Debug, Clone, Copy, Event)]
pub struct RemoveScenario(pub Entity);

/// When a scenario is removed, all child scenarios are removed as well
pub fn handle_remove_scenarios(
    mut commands: Commands,
    mut remove_scenario_requests: EventReader<RemoveScenario>,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut change_default_scenario: EventWriter<ChangeDefaultScenario>,
    mut create_new_scenario: EventWriter<CreateScenario>,
    mut delete: EventWriter<Delete>,
    mut scenarios: Query<
        (Entity, &Affiliation<Entity>, Option<&mut Dependents>),
        With<ScenarioModifiers<Entity>>,
    >,
    children: Query<&Children>,
    default_scenario: Res<DefaultScenario>,
) {
    for request in remove_scenario_requests.read() {
        // Any child scenarios are considered dependents to be deleted
        let mut subtree_dependents = HashSet::<Entity>::new();
        let mut queue = vec![request.0];
        while let Some(scenario_entity) = queue.pop() {
            if let Ok(children) = children.get(scenario_entity) {
                children.iter().for_each(|e| {
                    subtree_dependents.insert(e);
                    queue.push(e);
                });
            }
        }

        // If the default scenario has been removed, set default to its parent if any, otherwise None
        let new_default_scenario = if default_scenario.0 == Some(request.0) {
            let new_default_scenario = scenarios
                .get(request.0)
                .ok()
                .and_then(|(_, affiliation, _)| affiliation.0);
            change_default_scenario.write(ChangeDefaultScenario(new_default_scenario));
            new_default_scenario
        } else {
            default_scenario.0
        };

        // Change to DefaultScenario, else parent scenario, else root, else create an empty scenario and switch to it
        if let Some(default_scenario_entity) = new_default_scenario {
            change_current_scenario.write(ChangeCurrentScenario(default_scenario_entity));
        } else if let Some(parent_scenario_entity) =
            scenarios.get(request.0).map(|(_, a, _)| a.0).ok().flatten()
        {
            change_current_scenario.write(ChangeCurrentScenario(parent_scenario_entity));
        } else if let Some((root_scenario_entity, _, _)) = scenarios
            .iter()
            .filter(|(e, a, _)| request.0 != *e && a.0.is_none())
            .next()
        {
            change_current_scenario.write(ChangeCurrentScenario(root_scenario_entity));
        } else {
            create_new_scenario.write(CreateScenario {
                name: None,
                parent: None,
            });
        }

        // Delete with dependents
        if let Ok((_, _, Some(mut dependents))) = scenarios.get_mut(request.0) {
            dependents.extend(subtree_dependents.iter());
        } else {
            commands
                .entity(request.0)
                .insert(Dependents(subtree_dependents));
        }
        delete.write(Delete::new(request.0).and_dependents());
    }
}

/// Unique UUID to identify issue of hidden model instance
pub const HIDDEN_MODEL_INSTANCE_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x31923bdecb54473aa9a34b711423e9c1u128);

pub fn check_for_hidden_model_instances(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    get_modifier: GetModifier<Modifier<Inclusion>>,
    instances: Query<
        (Entity, &NameInSite, &Affiliation<Entity>),
        (With<ModelMarker>, Without<Group>),
    >,
    scenarios: Query<(Entity, &ScenarioModifiers<Entity>, &Affiliation<Entity>)>,
) {
    for root in validate_events.read() {
        for (instance_entity, instance_name, _) in instances.iter() {
            if count_scenarios_with_inclusion(&scenarios, instance_entity, &get_modifier) > 0 {
                continue;
            }
            let issue = Issue {
                key: IssueKey {
                    entities: [instance_entity].into(),
                    kind: HIDDEN_MODEL_INSTANCE_ISSUE_UUID,
                },
                brief: format!(
                    "Model instance {:?} is not included in any scenario",
                    instance_name
                ),
                hint: "Model instance is not present in any scenario. \
                      Check that the model instance is meant to be hidden from all scenarios."
                    .to_string(),
            };
            let issue_id = commands.spawn(issue).id();
            commands.entity(**root).add_child(issue_id);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::site::{
        AssociatedGraphs, Edge, Lane, Location, LocationTag, Motion, OrientationConstraint, Point,
        PropertyPlugin, ReverseLane, Trashcan, UpdateModifier,
    };
    use std::collections::BTreeSet;

    fn setup_scenario_test_app() -> App {
        let mut app = App::new();
        app.init_resource::<CurrentScenario>()
            .init_resource::<Trashcan>()
            .add_event::<ChangeCurrentScenario>()
            .add_plugins((
                PropertyPlugin::<Inclusion, LaneMarker>::default(),
                PropertyPlugin::<Motion, LaneMarker>::default(),
                PropertyPlugin::<ReverseLane, LaneMarker>::default(),
                PropertyPlugin::<AssociatedGraphs<Entity>, LaneMarker>::default(),
                PropertyPlugin::<Inclusion, LocationTags>::default(),
                PropertyPlugin::<LocationTags, LocationTags>::default(),
                PropertyPlugin::<AssociatedGraphs<Entity>, LocationTags>::default(),
            ))
            .add_systems(Update, update_current_scenario);
        app
    }

    #[test]
    fn test_lane_and_location_scenario_inheritance_override_and_reset() {
        let mut app = setup_scenario_test_app();

        let root_scenario = app
            .world_mut()
            .spawn(ScenarioBundle::<Entity>::new(Some("Root".into()), None))
            .id();
        app.update();
        assert_eq!(app.world().resource::<CurrentScenario>().0, Some(root_scenario));

        let a1 = app.world_mut().spawn_empty().id();
        let a2 = app.world_mut().spawn_empty().id();
        let graph1 = app.world_mut().spawn_empty().id();

        let lane = app
            .world_mut()
            .spawn(Lane::<Entity>::from(Edge::new(a1, a2)))
            .id();
        let location = app
            .world_mut()
            .spawn(Location::<Entity>::from(Point(a1)))
            .id();
        app.update();

        // Spawn a child scenario (on_add_scenario switches to child_scenario)
        let child_scenario = app
            .world_mut()
            .spawn(ScenarioBundle::<Entity>::new(
                Some("Child".into()),
                Some(root_scenario),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().resource::<CurrentScenario>().0,
            Some(child_scenario)
        );

        // Inherited in child scenario
        assert_eq!(*app.world().get::<Inclusion>(lane).unwrap(), Inclusion::Included);
        assert_eq!(*app.world().get::<ReverseLane>(lane).unwrap(), ReverseLane::Same);
        assert_eq!(
            *app.world().get::<Inclusion>(location).unwrap(),
            Inclusion::Included
        );
        assert!(app.world().get::<LocationTags>(location).unwrap().0.is_empty());

        // Override lane properties and location tags in child scenario
        let child_motion = Motion {
            orientation_constraint: OrientationConstraint::Forwards,
            speed_limit: Some(2.5),
            dock: None,
        };
        app.world_mut().entity_mut(lane).insert((
            child_motion.clone(),
            ReverseLane::Disable,
            AssociatedGraphs::Only(BTreeSet::from([graph1])),
        ));
        app.world_mut().entity_mut(location).insert((
            Inclusion::Hidden,
            LocationTags(vec![LocationTag::Charger]),
        ));
        app.update();

        // Switch back to root scenario and verify root values are preserved
        app.world_mut()
            .send_event(ChangeCurrentScenario(root_scenario));
        app.update();
        assert_eq!(*app.world().get::<Motion>(lane).unwrap(), Motion::default());
        assert_eq!(*app.world().get::<ReverseLane>(lane).unwrap(), ReverseLane::Same);
        assert_eq!(
            *app.world().get::<AssociatedGraphs<Entity>>(lane).unwrap(),
            AssociatedGraphs::All
        );
        assert_eq!(
            *app.world().get::<Inclusion>(location).unwrap(),
            Inclusion::Included
        );
        assert!(app.world().get::<LocationTags>(location).unwrap().0.is_empty());

        // Switch to child scenario and verify overrides are restored
        app.world_mut()
            .send_event(ChangeCurrentScenario(child_scenario));
        app.update();
        assert_eq!(*app.world().get::<Motion>(lane).unwrap(), child_motion);
        assert_eq!(
            *app.world().get::<ReverseLane>(lane).unwrap(),
            ReverseLane::Disable
        );
        assert_eq!(
            *app.world().get::<AssociatedGraphs<Entity>>(lane).unwrap(),
            AssociatedGraphs::Only(BTreeSet::from([graph1]))
        );
        assert_eq!(
            *app.world().get::<Inclusion>(location).unwrap(),
            Inclusion::Hidden
        );
        assert_eq!(
            app.world().get::<LocationTags>(location).unwrap().0,
            vec![LocationTag::Charger]
        );

        // Reset overrides in child scenario and verify fallback to root scenario values
        app.world_mut()
            .trigger(UpdateModifier::<Motion>::reset(child_scenario, lane));
        app.world_mut()
            .trigger(UpdateModifier::<ReverseLane>::reset(child_scenario, lane));
        app.world_mut()
            .trigger(UpdateModifier::<AssociatedGraphs<Entity>>::reset(
                child_scenario,
                lane,
            ));
        app.world_mut()
            .trigger(UpdateModifier::<Inclusion>::reset(child_scenario, location));
        app.world_mut()
            .trigger(UpdateModifier::<LocationTags>::reset(
                child_scenario,
                location,
            ));
        app.update();

        assert_eq!(*app.world().get::<Motion>(lane).unwrap(), Motion::default());
        assert_eq!(*app.world().get::<ReverseLane>(lane).unwrap(), ReverseLane::Same);
        assert_eq!(
            *app.world().get::<AssociatedGraphs<Entity>>(lane).unwrap(),
            AssociatedGraphs::All
        );
        assert_eq!(
            *app.world().get::<Inclusion>(location).unwrap(),
            Inclusion::Included
        );
        assert!(app.world().get::<LocationTags>(location).unwrap().0.is_empty());
    }

    #[test]
    fn test_creating_lane_or_location_in_child_scenario_hides_in_root() {
        let mut app = setup_scenario_test_app();

        let root_scenario = app
            .world_mut()
            .spawn(ScenarioBundle::<Entity>::new(Some("Root".into()), None))
            .id();
        app.update();

        let child_scenario = app
            .world_mut()
            .spawn(ScenarioBundle::<Entity>::new(
                Some("Child".into()),
                Some(root_scenario),
            ))
            .id();
        app.update();
        assert_eq!(
            app.world().resource::<CurrentScenario>().0,
            Some(child_scenario)
        );

        let a1 = app.world_mut().spawn_empty().id();
        let a2 = app.world_mut().spawn_empty().id();
        let lane = app
            .world_mut()
            .spawn(Lane::<Entity>::from(Edge::new(a1, a2)))
            .id();
        let location = app
            .world_mut()
            .spawn(Location::<Entity>::from(Point(a1)))
            .id();
        app.update();

        // In child scenario, newly spawned lane and location are Included,
        // and child scenario's modifier entity should NOT have spurious Motion/ReverseLane/LocationTags modifiers.
        assert_eq!(*app.world().get::<Inclusion>(lane).unwrap(), Inclusion::Included);
        assert_eq!(
            *app.world().get::<Inclusion>(location).unwrap(),
            Inclusion::Included
        );
        let child_modifiers = app
            .world()
            .get::<ScenarioModifiers<Entity>>(child_scenario)
            .unwrap();
        let child_lane_mod_entity = *child_modifiers.get(&lane).unwrap();
        assert!(app.world().get::<Modifier<Inclusion>>(child_lane_mod_entity).is_some());
        assert!(app.world().get::<Modifier<Motion>>(child_lane_mod_entity).is_none());
        assert!(app.world().get::<Modifier<ReverseLane>>(child_lane_mod_entity).is_none());

        // Switch to root scenario: lane and location created in child scenario must be Hidden in root!
        app.world_mut()
            .send_event(ChangeCurrentScenario(root_scenario));
        app.update();
        assert_eq!(*app.world().get::<Inclusion>(lane).unwrap(), Inclusion::Hidden);
        assert_eq!(
            *app.world().get::<Inclusion>(location).unwrap(),
            Inclusion::Hidden
        );
    }
}
