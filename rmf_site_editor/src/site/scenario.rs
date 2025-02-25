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
        AddedInstance, CurrentScenario, Delete, Dependents, HiddenInstance, Instance,
        InstanceMarker, ModifiedInstance, Pending, Pose, Scenario, ScenarioBundle, ScenarioMarker,
    },
    CurrentWorkspace,
};
use bevy::prelude::*;
use std::collections::HashMap;

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

pub enum UpdateInstanceType {
    Include,
    Hide,
    Modify,
    ResetPose,
}

#[derive(Event)]
pub struct UpdateInstance {
    pub scenario: Entity,
    pub instance: Entity,
    pub update_type: UpdateInstanceType,
}

/// Handles changes to the current scenario
pub fn update_current_scenario(
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    mut update_instance: EventWriter<UpdateInstance>,
    children: Query<&Children>,
    scenarios: Query<&Scenario<Entity>>,
    mut instances: Query<(Entity, &mut Pose, &mut Visibility), With<InstanceMarker>>,
) {
    if let Some(ChangeCurrentScenario(scenario_entity)) = change_current_scenario.read().last() {
        let Ok(scenario) = scenarios.get(*scenario_entity) else {
            error!("Failed to get scenario entity!");
            return;
        };

        for (entity, mut pose, mut visibility) in instances.iter_mut() {
            if let Some(new_pose) = scenario.instances.get(&entity).and_then(|i| match i {
                Instance::Added(added) => Some(added.pose),
                Instance::Modified(modified) => Some(modified.pose),
                _ => None,
            }) {
                *pose = new_pose.clone();
                *visibility = Visibility::Inherited;

                // Trigger an update for this instance in children scenarios (if any) since
                // the same instance in the parent scenario may have been modified
                if let Ok(scenario_children) = children.get(*scenario_entity) {
                    scenario_children.iter().for_each(|child| {
                        update_instance.send(UpdateInstance {
                            scenario: *child,
                            instance: entity,
                            update_type: UpdateInstanceType::Modify,
                        });
                    });
                }
            } else {
                *visibility = Visibility::Hidden;
            }
        }

        if let Some(entity) = selection.0 {
            if let Ok(mut selected) = selected.get_mut(entity) {
                let mut deselect = false;
                if scenario.instances.get(&entity).is_some_and(|i| match i {
                    Instance::Hidden(_) => true,
                    _ => false,
                }) {
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
    mut update_instance: EventWriter<UpdateInstance>,
    changed_instances: Query<(Entity, Ref<Pose>), (With<InstanceMarker>, Without<Pending>)>,
    children: Query<&Children>,
) {
    // Do nothing if scenario has changed, as we rely on pose changes by the user and not the system updating instances
    for ChangeCurrentScenario(_) in change_current_scenario.read() {
        return;
    }
    let Some(current_scenario_entity) = current_scenario.0 else {
        return;
    };
    let Ok(mut current_scenario) = scenarios.get_mut(current_scenario_entity) else {
        return;
    };

    let mut newly_added_instances = HashMap::new();
    let parent_exists = current_scenario.parent_scenario.0.is_some();
    for (entity, new_pose) in changed_instances.iter() {
        if new_pose.is_changed() {
            if let Some(instance) = current_scenario.instances.get_mut(&entity) {
                *instance = if parent_exists {
                    Instance::Modified(ModifiedInstance {
                        pose: new_pose.clone(),
                    })
                } else {
                    Instance::Added(AddedInstance {
                        pose: new_pose.clone(),
                    })
                };
                // Update children scenarios/instances since the parent pose changed
                if let Ok(scenario_children) = children.get(current_scenario_entity) {
                    scenario_children.iter().for_each(|child| {
                        update_instance.send(UpdateInstance {
                            scenario: *child,
                            instance: entity,
                            update_type: UpdateInstanceType::Modify,
                        });
                    });
                }
            } else if new_pose.is_added() {
                newly_added_instances.insert(entity, new_pose.clone());
                current_scenario.instances.insert(
                    entity,
                    Instance::Added(AddedInstance {
                        pose: new_pose.clone(),
                    }),
                );
            }
        }
    }

    // Add any newly created instance from the current scenario to all others scenarios hidden
    for (entity, pose) in newly_added_instances.drain() {
        for mut scenario in scenarios.iter_mut() {
            if scenario.instances.contains_key(&entity) {
                continue;
            }
            scenario
                .instances
                .insert(entity, Instance::Hidden(HiddenInstance { pose }));
        }
    }
}

/// Checks that the current scenario's included instances are categorized correctly
pub fn handle_instance_updates(
    current_scenario: Res<CurrentScenario>,
    mut scenarios: Query<&mut Scenario<Entity>>,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut update_instance: EventReader<UpdateInstance>,
    parents: Query<&Parent>,
) {
    for update in update_instance.read() {
        let parent_pose = parents
            .get(update.scenario)
            .and_then(|p| scenarios.get(p.get()))
            .ok()
            .and_then(|ps| ps.instances.get(&update.instance))
            .and_then(|instance| match instance {
                Instance::Added(added) => Some(added.pose),
                Instance::Modified(modified) => Some(modified.pose),
                // Even if parent pose is hidden, we still allow reset
                Instance::Hidden(hidden) => Some(hidden.pose),
            });

        let Ok(mut scenario) = scenarios.get_mut(update.scenario) else {
            return;
        };
        let Some(instance) = scenario.instances.get_mut(&update.instance) else {
            continue;
        };
        let instance_pose = match instance {
            Instance::Added(added) => added.pose,
            Instance::Modified(modified) => modified.pose,
            Instance::Hidden(hidden) => hidden.pose,
        };

        match update.update_type {
            UpdateInstanceType::Include => {
                if parent_pose.is_some_and(|p| p != instance_pose) {
                    *instance = Instance::Modified(ModifiedInstance {
                        pose: instance_pose,
                    });
                } else {
                    *instance = Instance::Added(AddedInstance {
                        pose: instance_pose,
                    });
                }
            }
            UpdateInstanceType::Hide => {
                *instance = Instance::Hidden(HiddenInstance {
                    pose: instance_pose,
                });
            }
            UpdateInstanceType::Modify => {
                // If the instance pose in this scenario's parent has changed, update the instance in
                // this scenario to Modified
                if parent_pose.is_some_and(|p| p != instance_pose) {
                    *instance = Instance::Modified(ModifiedInstance {
                        pose: instance_pose,
                    });
                }
            }
            UpdateInstanceType::ResetPose => {
                if let Some(reset_pose) = parent_pose {
                    *instance = Instance::Added(AddedInstance { pose: reset_pose });
                };
            }
        }

        if current_scenario.0.is_some_and(|e| e == update.scenario) {
            change_current_scenario.send(ChangeCurrentScenario(update.scenario));
        };
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
        // Any child scenarios are considered dependents to be deleted
        let mut subtree_dependents = std::collections::HashSet::<Entity>::new();
        let mut queue = vec![request.0];
        while let Some(scenario_entity) = queue.pop() {
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

        // Remove relationship with parent (if any) before deletion
        commands.entity(request.0).remove_parent();

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
