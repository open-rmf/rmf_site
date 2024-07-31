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
    interaction::Selection,
    site::{CurrentScenario, Delete, InstanceMarker, Pending, Pose, Scenario, SiteParent},
    CurrentWorkspace,
};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

pub fn update_current_scenario(
    mut commands: Commands,
    mut selection: ResMut<Selection>,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    current_workspace: Res<CurrentWorkspace>,
    scenarios: Query<&Scenario<Entity>>,
    mut instances: Query<
        (Entity, &mut Pose, &SiteParent<Entity>, &mut Visibility),
        With<InstanceMarker>,
    >,
) {
    for ChangeCurrentScenario(scenario_entity) in change_current_scenario.read() {
        // Used to build a scenario from root
        let mut scenario_stack = Vec::<&Scenario<Entity>>::new();
        let mut scenario = scenarios
            .get(*scenario_entity)
            .expect("Failed to get scenario entity");
        loop {
            scenario_stack.push(scenario);
            if let Some(scenario_parent) = scenario.parent_scenario.0 {
                scenario = scenarios
                    .get(scenario_parent)
                    .expect("Scenario parent doesn't exist");
            } else {
                break;
            }
        }

        // Iterate stack to identify instances in this model
        let mut active_instances = HashMap::<Entity, Pose>::new();
        for scenario in scenario_stack.iter().rev() {
            for (e, pose) in scenario.added_instances.iter() {
                active_instances.insert(*e, pose.clone());
            }
            for (e, pose) in scenario.moved_instances.iter() {
                active_instances.insert(*e, pose.clone());
            }
            for e in scenario.removed_instances.iter() {
                active_instances.remove(e);
            }
        }

        let current_site_entity = match current_workspace.root {
            Some(current_site) => current_site,
            None => return,
        };

        // If active, assign parent to level, otherwise assign parent to site
        for (entity, mut pose, parent, mut visibility) in instances.iter_mut() {
            if let Some(new_pose) = active_instances.get(&entity) {
                if let Some(parent_entity) = parent.0 {
                    commands.entity(entity).set_parent(parent_entity);
                } else {
                    commands.entity(entity).set_parent(current_site_entity);
                    warn!("Model instance {:?} has no valid site parent", entity);
                }
                *pose = new_pose.clone();
                *visibility = Visibility::Inherited;
            } else {
                commands.entity(entity).set_parent(current_site_entity);
                *visibility = Visibility::Hidden;
            }
        }

        // Deselect if not in current scenario
        if let Some(selected_entity) = selection.0.clone() {
            if let Ok((instance_entity, ..)) = instances.get(selected_entity) {
                if active_instances.get(&instance_entity).is_none() {
                    selection.0 = None;
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

    if let Some(mut current_scenario) = current_scenario
        .0
        .and_then(|entity| scenarios.get_mut(entity).ok())
    {
        for (entity, pose) in changed_instances.iter() {
            if pose.is_changed() {
                let existing_removed_instance = current_scenario
                    .removed_instances
                    .iter_mut()
                    .find(|e| **e == entity)
                    .map(|e| e.clone());
                if let Some(existing_removed_instance) = existing_removed_instance {
                    current_scenario
                        .moved_instances
                        .retain(|(e, _)| *e != existing_removed_instance);
                    current_scenario
                        .added_instances
                        .retain(|(e, _)| *e != existing_removed_instance);
                    return;
                }

                let existing_added_instance: Option<&mut (Entity, Pose)> = current_scenario
                    .added_instances
                    .iter_mut()
                    .find(|(e, _)| *e == entity);
                if let Some(existing_added_instance) = existing_added_instance {
                    existing_added_instance.1 = pose.clone();
                    return;
                } else if pose.is_added() {
                    current_scenario
                        .added_instances
                        .push((entity, pose.clone()));
                    return;
                }

                let existing_moved_instance = current_scenario
                    .moved_instances
                    .iter_mut()
                    .find(|(e, _)| *e == entity);
                if let Some(existing_moved_instance) = existing_moved_instance {
                    existing_moved_instance.1 = pose.clone();
                    return;
                } else {
                    current_scenario
                        .moved_instances
                        .push((entity, pose.clone()));
                    return;
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Event)]
pub struct RemoveInstance(pub Entity);

/// Handle requests to remove model instances. If an instance was added in this scenario, or if
/// the scenario is root, the InstanceMarker is removed, allowing it to be permanently deleted.
/// Otherwise, it is only temporarily removed.
pub fn remove_instances(
    mut commands: Commands,
    mut scenarios: Query<&mut Scenario<Entity>>,
    current_scenario: ResMut<CurrentScenario>,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut removals: EventReader<RemoveInstance>,
    mut delete: EventWriter<Delete>,
) {
    for removal in removals.read() {
        let Some(current_scenario_entity) = current_scenario.0 else {
            delete.send(Delete::new(removal.0));
            return;
        };

        if let Ok(mut current_scenario) = scenarios.get_mut(current_scenario_entity) {
            // Delete if root scenario
            if current_scenario.parent_scenario.0.is_none() {
                current_scenario
                    .added_instances
                    .retain(|(e, _)| *e != removal.0);
                commands.entity(removal.0).remove::<InstanceMarker>();
                delete.send(Delete::new(removal.0));
                return;
            }
            // Delete if added in this scenario
            if let Some(added_id) = current_scenario
                .added_instances
                .iter()
                .position(|(e, _)| *e == removal.0)
            {
                current_scenario.added_instances.remove(added_id);
                commands.entity(removal.0).remove::<InstanceMarker>();
                delete.send(Delete::new(removal.0));
                return;
            }
            // Otherwise, remove
            current_scenario
                .moved_instances
                .retain(|(e, _)| *e != removal.0);
            current_scenario.removed_instances.push(removal.0);
            change_current_scenario.send(ChangeCurrentScenario(current_scenario_entity));
        }
    }
}
