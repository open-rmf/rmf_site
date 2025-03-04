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
        IssueKey, ModelMarker, NameInSite, Pending, Pose, RecallInstance, Scenario, ScenarioBundle,
        ScenarioMarker,
    },
    widgets::view_model_instances::count_scenarios,
    CurrentWorkspace, Issue, ValidateWorkspace,
};
use bevy::{prelude::*, utils::Uuid};
use std::collections::HashSet;

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

#[derive(Clone, Debug, Event)]
pub struct CreateScenario {
    pub name: Option<String>,
    pub parent: Option<Entity>,
}

#[derive(Clone, Debug)]
pub enum UpdateInstanceType {
    Include,
    Hide,
    Add(Pose),
    Modify(Pose),
    ResetPose,
}

#[derive(Clone, Debug, Event)]
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
    mut instances: Query<(Entity, &NameInSite, &mut Pose, &mut Visibility), With<InstanceMarker>>,
    children: Query<&Children>,
    recall_instance: Query<&RecallInstance>,
    scenarios: Query<(Entity, &mut Scenario<Entity>)>,
    scenario_entities: Query<(&mut Instance, &Affiliation<Entity>)>,
) {
    for ChangeCurrentScenario(scenario_entity) in change_current_scenario.read() {
        let mut deselect = false;
        let instance_entities =
            get_scenario_instance_entities(*scenario_entity, &children, &scenario_entities);

        // Loop over every instance in this site
        for (entity, name, mut pose, mut visibility) in instances.iter_mut() {
            let Some((instance, _)) = instance_entities
                .iter()
                .find(|(_, i)| *i == entity)
                .and_then(|(c_entity, _)| scenario_entities.get(*c_entity).ok())
            else {
                // No child Instance matches this instance entity, set to hidden
                *visibility = Visibility::Hidden;
                if selection.0.is_some_and(|e| e == entity) {
                    deselect = true;
                }
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
                        .or(retrieve_parent_pose(entity, *scenario_entity, &children, &recall_instance, &scenarios, &scenario_entities))
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
                Instance::Hidden => {
                    *visibility = Visibility::Hidden;
                    if selection.0.is_some_and(|e| e == entity) {
                        deselect = true;
                    }
                }
            }
        }

        if let Some(mut selected) = selection.0.and_then(|e| selected.get_mut(e).ok()) {
            if current_scenario.0.is_some_and(|e| e != *scenario_entity) {
                // Deselect if scenario has changed
                deselect = true;
            }

            if deselect {
                selection.0 = None;
                selected.is_selected = false;
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
    children: &Query<&Children>,
    recall_instance: &Query<&RecallInstance>,
    scenarios: &Query<(Entity, &mut Scenario<Entity>)>,
    scenario_entities: &Query<(&mut Instance, &Affiliation<Entity>)>,
) -> Option<Pose> {
    let mut parent_pose: Option<Pose> = None;
    let mut entity = scenario_entity;
    while parent_pose.is_none() {
        let Ok((_, scenario)) = scenarios.get(entity) else {
            break;
        };
        let Some((parent_entity, _)) = scenario
            .parent_scenario
            .0
            .and_then(|e| scenarios.get(e).ok())
        else {
            break;
        };

        let instance_entities =
            get_scenario_instance_entities(parent_entity, children, scenario_entities);

        if let Some((scenario_child, _)) = instance_entities
            .iter()
            .find(|(_, i)| *i == instance_entity)
        {
            parent_pose = scenario_entities
                .get(*scenario_child)
                .ok()
                .and_then(|(i, _)| {
                    i.pose().or(recall_instance
                        .get(*scenario_child)
                        .ok()
                        .and_then(|r| r.pose))
                });
        }
        entity = parent_entity;
    }
    parent_pose
}

/// This system current searches for scenario children entities with the Instance component
/// TODO(@xiyuoh) generalize this at some point to use T
pub fn get_scenario_instance_entities(
    entity: Entity,
    children: &Query<&Children>,
    scenario_entities: &Query<(&mut Instance, &Affiliation<Entity>)>,
) -> Vec<(Entity, Entity)> {
    let mut scenario_instances = Vec::new();
    if let Ok(scenario_children) = children.get(entity) {
        for child in scenario_children.iter() {
            if let Some(affiliated_entity) =
                scenario_entities.get(*child).ok().and_then(|(_, a)| a.0)
            {
                scenario_instances.push((*child, affiliated_entity));
            }
        }
    };
    scenario_instances
}

/// Checks that the current scenario's included instances are categorized correctly
pub fn handle_instance_updates(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut scenario_instances: Query<(&mut Instance, &Affiliation<Entity>)>,
    mut update_instance: EventReader<UpdateInstance>,
    children: Query<&Children>,
    current_scenario: Res<CurrentScenario>,
    scenarios: Query<(Entity, &mut Scenario<Entity>)>,
    model_instances: Query<&NameInSite, With<InstanceMarker>>,
    recall_instance: Query<&RecallInstance>,
) {
    for update in update_instance.read() {
        let parent_pose = retrieve_parent_pose(
            update.instance,
            update.scenario,
            &children,
            &recall_instance,
            &scenarios,
            &scenario_instances,
        );
        match update.update_type {
            UpdateInstanceType::Add(new_pose) => {
                commands
                    .spawn(Instance::new_added(new_pose.clone()))
                    .insert(Affiliation(Some(update.instance)))
                    .set_parent(update.scenario);
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
                    if let Ok((child_entity, _)) = scenarios.get(dependent) {
                        commands
                            .spawn(Instance::new_inherited(None))
                            .insert(Affiliation(Some(update.instance)))
                            .set_parent(child_entity);
                    }
                }
            }
            _ => {
                let instance_entities =
                    get_scenario_instance_entities(update.scenario, &children, &scenario_instances);
                let Some(((mut instance, _), scenario_child)) = instance_entities
                    .iter()
                    .find(|(_, i)| *i == update.instance)
                    .and_then(|(c_entity, _)| {
                        scenario_instances
                            .get_mut(*c_entity)
                            .ok()
                            .zip(Some(c_entity))
                    })
                else {
                    continue;
                };
                let instance = instance.as_mut();

                match update.update_type {
                    UpdateInstanceType::Include => {
                        let instance_pose = instance.pose().or(recall_instance
                            .get(*scenario_child)
                            .ok()
                            .and_then(|r| r.pose));

                        if parent_pose.is_some() {
                            *instance = Instance::new_inherited(instance_pose)
                        } else if let Some(instance_pose) = instance_pose {
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
                        *instance = Instance::new_hidden();
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
    mut create_new_scenario: EventWriter<CreateScenario>,
    mut delete: EventWriter<Delete>,
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
            create_new_scenario.send(CreateScenario {
                name: None,
                parent: None,
            });
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
    children: Query<&Children>,
    instances: Query<
        (Entity, &NameInSite, &Affiliation<Entity>),
        (With<ModelMarker>, Without<Group>),
    >,
    scenarios: Query<(Entity, &NameInSite, &mut Scenario<Entity>), With<ScenarioMarker>>,
    scenario_entities: Query<(&mut Instance, &Affiliation<Entity>)>,
) {
    for root in validate_events.read() {
        for (instance_entity, instance_name, _) in instances.iter() {
            if count_scenarios(&scenarios, instance_entity, &children, &scenario_entities) > 0 {
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

/// Create a new scenario and its children entities
pub fn handle_create_scenarios(
    mut commands: Commands,
    mut new_scenarios: EventReader<CreateScenario>,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    children: Query<&Children>,
    current_workspace: Res<CurrentWorkspace>,
    instances: Query<(&Instance, &Affiliation<Entity>)>,
) {
    for new in new_scenarios.read() {
        let mut cmd = if let Some(name) = &new.name {
            commands.spawn(ScenarioBundle::<Entity>::from_name_parent(
                name.clone(),
                new.parent,
            ))
        } else {
            commands.spawn(ScenarioBundle::<Entity>::default())
        };
        let scenario_entity = cmd.id();

        if let Some(parent_scenario_entity) = new.parent {
            cmd.set_parent(parent_scenario_entity);

            // Inherit any children entities with Instance component from the parent scenario
            if let Ok(children) = children.get(parent_scenario_entity) {
                children.iter().for_each(|e| {
                    if let Ok((instance, affiliation)) = instances.get(*e).map(|(i, a)| match i {
                        Instance::Added(_) | Instance::Inherited(_) => {
                            (Instance::new_inherited(None), a.clone())
                        }
                        Instance::Hidden => (i.clone(), a.clone()),
                    }) {
                        commands
                            .spawn(instance)
                            .insert(affiliation)
                            .set_parent(scenario_entity);
                    }
                });
            }
        } else if let Some(site_entity) = current_workspace.root {
            cmd.set_parent(site_entity);
        }

        change_current_scenario.send(ChangeCurrentScenario(scenario_entity));
    }
}
