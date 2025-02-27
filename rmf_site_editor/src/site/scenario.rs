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
        Affiliation, CurrentScenario, Delete, Dependents, Group, Instance, InstanceMarker,
        IssueKey, ModelMarker, NameInSite, Pending, Pose, Scenario, ScenarioBundle, ScenarioMarker,
    },
    widgets::view_model_instances::count_scenarios,
    CurrentWorkspace, Issue, ValidateWorkspace,
};
use bevy::{prelude::*, utils::Uuid};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

pub enum UpdateInstanceType {
    Include,
    Hide,
    Add(Pose),
    Modify(Pose),
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
    scenarios: Query<(Entity, &mut Scenario<Entity>)>,
    mut instances: Query<(Entity, &NameInSite, &mut Pose, &mut Visibility), With<InstanceMarker>>,
) {
    if let Some(ChangeCurrentScenario(scenario_entity)) = change_current_scenario.read().last() {
        let Ok((_, scenario)) = scenarios.get(*scenario_entity) else {
            error!("Failed to get scenario entity!");
            return;
        };

        for (entity, name, mut pose, mut visibility) in instances.iter_mut() {
            let Some(instance) = scenario.instances.get(&entity) else {
                *visibility = Visibility::Hidden;
                continue;
            };

            match instance {
                Instance::Added(added) => {
                    *pose = added.pose.clone();
                    *visibility = Visibility::Inherited;
                }
                Instance::Inherited(inherited) => {
                    *pose = inherited
                        .modified_pose
                        .or(retrieve_parent_pose(entity, *scenario_entity, &scenarios))
                        .map_or_else(
                            || {
                                error!(
                                    "Instance {:?} is included in the current scenario, but no pose found! \
                                    Setting instance to default pose in current scenario.",
                                    name.0
                                );
                                Pose::default()
                            },
                            |p| p.clone(),
                        );
                    *visibility = Visibility::Inherited;
                }
                Instance::Hidden(_) => {
                    *visibility = Visibility::Hidden;
                }
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
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut update_instance: EventWriter<UpdateInstance>,
    changed_instances: Query<(Entity, Ref<Pose>), (With<InstanceMarker>, Without<Pending>)>,
) {
    // Do nothing if scenario has changed, as we rely on pose changes by the user and not the system updating instances
    for ChangeCurrentScenario(_) in change_current_scenario.read() {
        return;
    }
    let Some(current_scenario_entity) = current_scenario.0 else {
        return;
    };

    for (entity, new_pose) in changed_instances.iter() {
        if new_pose.is_added() {
            update_instance.send(UpdateInstance {
                scenario: current_scenario_entity,
                instance: entity,
                update_type: UpdateInstanceType::Add(new_pose.clone()),
            });
        } else if new_pose.is_changed() {
            update_instance.send(UpdateInstance {
                scenario: current_scenario_entity,
                instance: entity,
                update_type: UpdateInstanceType::Modify(new_pose.clone()),
            });
        }
    }
}

fn retrieve_parent_pose(
    instance_entity: Entity,
    scenario_entity: Entity,
    scenarios: &Query<(Entity, &mut Scenario<Entity>)>,
) -> Option<Pose> {
    let mut parent_pose: Option<Pose> = None;
    let mut entity = scenario_entity;
    while parent_pose.is_none() {
        let Ok((_, scenario)) = scenarios.get(entity) else {
            break;
        };
        let Some((parent_entity, parent_scenario)) = scenario
            .parent_scenario
            .0
            .and_then(|e| scenarios.get(e).ok())
        else {
            break;
        };

        entity = parent_entity;
        parent_pose = parent_scenario
            .instances
            .get(&instance_entity)
            .and_then(|instance| instance.pose());
    }
    parent_pose
}

/// Checks that the current scenario's included instances are categorized correctly
pub fn handle_instance_updates(
    current_scenario: Res<CurrentScenario>,
    mut scenarios: Query<(Entity, &mut Scenario<Entity>)>,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut update_instance: EventReader<UpdateInstance>,
    model_instances: Query<&NameInSite, With<InstanceMarker>>,
    children: Query<&Children>,
) {
    for update in update_instance.read() {
        let parent_pose = retrieve_parent_pose(update.instance, update.scenario, &scenarios);
        let Ok((_, mut scenario)) = scenarios.get_mut(update.scenario) else {
            continue;
        };

        match update.update_type {
            UpdateInstanceType::Add(new_pose) => {
                scenario
                    .instances
                    .insert(update.instance, Instance::new_added(new_pose.clone()));
                // Insert this new instance into children scenarios as Inherited
                let mut subtree_dependents = HashSet::<Entity>::new();
                let mut queue = vec![update.scenario];
                while let Some(scenario_entity) = queue.pop() {
                    if let Ok(children) = children.get(scenario_entity) {
                        children.iter().for_each(|e| {
                            subtree_dependents.insert(*e);
                            queue.push(*e);
                        });
                    }
                }
                // Only insert new instance in children scenarios. Other parent/root
                // scenarios will not have access to this instance
                for dependent in subtree_dependents.drain() {
                    if let Ok((_, mut child_scenario)) = scenarios.get_mut(dependent) {
                        child_scenario
                            .instances
                            .insert(update.instance, Instance::new_inherited(None));
                    }
                }
            }
            _ => {
                let Some(instance) = scenario.instances.get_mut(&update.instance) else {
                    continue;
                };

                match update.update_type {
                    UpdateInstanceType::Include => {
                        if parent_pose.is_some() {
                            *instance = Instance::new_inherited(instance.pose())
                        } else if let Some(instance_pose) = instance.pose() {
                            *instance = Instance::new_added(instance_pose);
                        } else {
                            let instance_id = model_instances
                                .get(update.instance)
                                .map(|n| n.0.clone())
                                .unwrap_or(format!("{}", update.instance.index()).to_string());
                            error!(
                                "Unable to retrieve pose for instance {:?}, \
                                setting to default pose as AddedInstance in current scenario",
                                instance_id
                            );
                            *instance = Instance::new_added(Pose::default());
                        }
                    }
                    UpdateInstanceType::Hide => {
                        *instance = Instance::new_hidden(instance.pose());
                    }
                    UpdateInstanceType::Modify(new_pose) => {
                        // Update Pose changes
                        match instance {
                            Instance::Added(_) => *instance = Instance::new_added(new_pose.clone()),
                            Instance::Inherited(_) => {
                                *instance = Instance::new_inherited(Some(new_pose.clone()))
                            }
                            _ => {}
                        }
                    }
                    UpdateInstanceType::ResetPose => {
                        if parent_pose.is_some() {
                            *instance = Instance::new_inherited(None)
                        }
                    }
                    _ => {}
                }
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

/// Unique UUID to identify issue of hidden model instance
pub const HIDDEN_MODEL_INSTANCE_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x31923bdecb54473aa9a34b711423e9c1u128);

pub fn check_for_hidden_model_instances(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    instances: Query<
        (Entity, &NameInSite, &Affiliation<Entity>),
        (With<ModelMarker>, Without<Group>),
    >,
    scenarios: Query<(Entity, &NameInSite, &mut Scenario<Entity>), With<ScenarioMarker>>,
) {
    for root in validate_events.read() {
        for (instance_entity, instance_name, _) in instances.iter() {
            if count_scenarios(&scenarios, instance_entity) > 0 {
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
