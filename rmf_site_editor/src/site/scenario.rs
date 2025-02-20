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
    interaction::{Selected, Selection},
    site::{
        CurrentScenario, Delete, Dependents, InstanceMarker, Pending, Pose, Scenario,
        ScenarioBundle, ScenarioMarker,
    },
    CurrentWorkspace,
};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

#[derive(Event)]
pub struct ResetPose(pub Entity);

/// Handles changes to the current scenario
pub fn update_current_scenario(
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    scenarios: Query<&Scenario<Entity>>,
    mut instances: Query<(Entity, &mut Pose, &mut Visibility), With<InstanceMarker>>,
) {
    if let Some(ChangeCurrentScenario(scenario_entity)) = change_current_scenario.read().last() {
        let Ok(scenario) = scenarios.get(*scenario_entity) else {
            error!("Failed to get scenario entity!");
            return;
        };

        for (entity, mut pose, mut visibility) in instances.iter_mut() {
            if let Some(((new_pose, _), _)) = scenario
                .instances
                .get(&entity)
                .filter(|(_, included)| *included)
            {
                *pose = new_pose.clone();
                *visibility = Visibility::Inherited;
            } else {
                *visibility = Visibility::Hidden;
            }
        }

        if let Some(entity) = selection.0 {
            if let Ok(mut selected) = selected.get_mut(entity) {
                let mut deselect = false;
                if !scenario
                    .instances
                    .get(&entity)
                    .is_some_and(|(_, included)| *included)
                {
                    // Deselect if model instance is hidden
                    deselect = true;
                } else if current_scenario.0.is_some_and(|e| e != *scenario_entity) {
                    // Deselect if scenario has changed
                    deselect = true;
                }

                if deselect {
                    selection.0 = None;
                    selected.is_selected = false;
                }
            }
        }

        *current_scenario = CurrentScenario(Some(*scenario_entity));
    }
}

/// Tracks pose changes for instances in the current scenario to update its properties
pub fn update_scenario_properties(
    current_scenario: Res<CurrentScenario>,
    mut scenarios: Query<&mut Scenario<Entity>>,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    changed_instances: Query<(Entity, Ref<Pose>), (With<InstanceMarker>, Without<Pending>)>,
) {
    // Do nothing if scenario has changed, as we rely on pose changes by the user and not the system updating instances
    for ChangeCurrentScenario(_) in change_current_scenario.read() {
        return;
    }

    let mut newly_added_instances = HashMap::new();
    if let Some(mut current_scenario) = current_scenario
        .0
        .and_then(|entity| scenarios.get_mut(entity).ok())
    {
        let parent_exists = current_scenario.parent_scenario.0.is_some();
        for (entity, new_pose) in changed_instances.iter() {
            if new_pose.is_changed() {
                if let Some(((current_pose, moved), _)) =
                    current_scenario.instances.get_mut(&entity)
                {
                    *current_pose = new_pose.clone();
                    *moved = parent_exists;
                } else if new_pose.is_added() {
                    newly_added_instances.insert(entity, new_pose.clone());
                    current_scenario
                        .instances
                        .insert(entity, ((new_pose.clone(), false), true));
                }
            }
        }
    }

    // Add any newly created instance from the current scenario to all others scenarios hidden
    for (entity, pose) in newly_added_instances.drain() {
        for mut scenario in scenarios.iter_mut() {
            if scenario.instances.contains_key(&entity) {
                continue;
            }
            scenario.instances.insert(entity, ((pose, false), false));
        }
    }
}

pub fn handle_reset_pose(
    current_scenario: Res<CurrentScenario>,
    mut scenarios: Query<&mut Scenario<Entity>>,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut reset_pose: EventReader<ResetPose>,
    parents: Query<&Parent>,
) {
    for ResetPose(instance_entity) in reset_pose.read() {
        let Some(current_scenario_entity) = current_scenario.0 else {
            continue;
        };
        let Some(((parent_pose, _), _)) = parents
            .get(current_scenario_entity)
            .and_then(|p| scenarios.get(p.get()))
            .ok()
            .and_then(|parent_scenario| parent_scenario.instances.get(instance_entity).cloned())
        else {
            continue;
        };
        let Ok(mut current_scenario) = scenarios.get_mut(current_scenario_entity) else {
            continue;
        };
        if let Some(((instance_pose, moved), _)) =
            current_scenario.instances.get_mut(instance_entity)
        {
            *instance_pose = parent_pose;
            *moved = false;
            change_current_scenario.send(ChangeCurrentScenario(current_scenario_entity));
        }
    }
}

#[derive(Debug, Clone, Copy, Event)]
pub struct RemoveScenario(pub Entity);

/// When a scenario is removed, all child scenarios are removed as well
pub fn handle_remove_scenarios(
    mut commands: Commands,
    mut remove_scenario_requests: EventReader<RemoveScenario>,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut delete: EventWriter<Delete>,
    mut current_scenario: ResMut<CurrentScenario>,
    current_workspace: Res<CurrentWorkspace>,
    mut scenarios: Query<
        (Entity, &Scenario<Entity>, Option<&mut Dependents>),
        With<ScenarioMarker>,
    >,
    children: Query<&Children>,
) {
    for request in remove_scenario_requests.read() {
        // Any child scenarios or instances added within the subtree are considered dependents
        // to be deleted
        let mut subtree_dependents = std::collections::HashSet::<Entity>::new();
        let mut queue = vec![request.0];
        while let Some(scenario_entity) = queue.pop() {
            if let Ok((_, scenario, _)) = scenarios.get(scenario_entity) {
                scenario.instances.iter().for_each(|(e, _)| {
                    subtree_dependents.insert(*e);
                });
            }
            if let Ok(children) = children.get(scenario_entity) {
                children.iter().for_each(|e| {
                    subtree_dependents.insert(*e);
                    queue.push(*e);
                });
            }
        }

        // Change to parent scenario, else root, else create an empty scenario and switch to it
        if let Some(parent_scenario_entity) = scenarios
            .get(request.0)
            .map(|(_, s, _)| s.parent_scenario.0)
            .ok()
            .flatten()
        {
            change_current_scenario.send(ChangeCurrentScenario(parent_scenario_entity));
        } else if let Some((root_scenario_entity, _, _)) = scenarios
            .iter()
            .filter(|(e, s, _)| request.0 != *e && s.parent_scenario.0.is_none())
            .next()
        {
            change_current_scenario.send(ChangeCurrentScenario(root_scenario_entity));
        } else {
            let new_scenario_entity = commands
                .spawn(ScenarioBundle::<Entity>::default())
                .set_parent(current_workspace.root.expect("No current site"))
                .id();
            *current_scenario = CurrentScenario(Some(new_scenario_entity));
        }

        // Delete with dependents
        if let Ok((_, _, Some(mut dependents))) = scenarios.get_mut(request.0) {
            dependents.extend(subtree_dependents.iter());
        } else {
            commands
                .entity(request.0)
                .insert(Dependents(subtree_dependents));
        }
        delete.send(Delete::new(request.0).and_dependents());
    }
}
