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
    interaction::{Select, Selection},
    site::{
        Affiliation, CurrentScenario, Delete, Dependents, Group, InstanceMarker, InstanceModifier,
        IssueKey, ModelMarker, NameInSite, Pending, Pose, RecallInstance, ScenarioBundle,
        ScenarioMarker,
    },
    widgets::view_model_instances::count_scenarios,
    CurrentWorkspace, Issue, ValidateWorkspace,
};
use bevy::{prelude::*, utils::Uuid};
use std::collections::{HashMap, HashSet};

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

#[derive(Clone, Debug, Event)]
pub struct CreateScenario {
    pub name: Option<String>,
    pub parent: Option<Entity>,
}

#[derive(Clone, Debug)]
pub enum UpdateInstance {
    Include,
    Hide,
    Modify(Pose),
    ResetPose,
}

#[derive(Clone, Debug, Event)]
pub struct UpdateInstanceEvent {
    pub scenario: Entity,
    pub instance: Entity,
    pub update: UpdateInstance,
}

/// Handles changes to the current scenario
pub fn update_current_scenario(
    mut select: EventWriter<Select>,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    mut instances: Query<(Entity, &NameInSite, &mut Pose, &mut Visibility), With<InstanceMarker>>,
    children: Query<&Children>,
    instance_modifiers: Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
    recall_instance: Query<&RecallInstance>,
    scenarios: Query<(Entity, &Affiliation<Entity>), With<ScenarioMarker>>,
    selection: Res<Selection>,
) {
    if let Some(ChangeCurrentScenario(scenario_entity)) = change_current_scenario.read().last() {
        let mut deselect = false;
        let instance_modifier_entities =
            get_instance_modifier_entities(*scenario_entity, &children, &instance_modifiers);

        // Loop over every model instance in this site
        for (entity, name, mut pose, mut visibility) in instances.iter_mut() {
            let Some((instance_modifier, _)) = instance_modifier_entities
                .get(&entity)
                .and_then(|modifier_entity| instance_modifiers.get(*modifier_entity).ok())
            else {
                // No instance modifier matches this model instance entity, set to hidden
                *visibility = Visibility::Hidden;
                if selection.0.is_some_and(|e| e == entity) {
                    deselect = true;
                }
                continue;
            };

            match instance_modifier {
                InstanceModifier::Added(added) => {
                    *pose = added.pose.clone();
                    *visibility = Visibility::Inherited;
                }
                InstanceModifier::Inherited(inherited) => {
                    *pose = inherited
                        .modified_pose
                        .or_else(|| retrieve_parent_pose(entity, *scenario_entity, &children, &recall_instance, &scenarios, &instance_modifiers))
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
                InstanceModifier::Hidden => {
                    *visibility = Visibility::Hidden;
                    if selection.0.is_some_and(|e| e == entity) {
                        deselect = true;
                    }
                }
            }
        }

        if deselect {
            select.send(Select::new(None));
        }

        *current_scenario = CurrentScenario(Some(*scenario_entity));
    }
}

/// Tracks pose changes for instances in the current scenario to update its properties
pub fn update_scenario_properties(
    current_scenario: Res<CurrentScenario>,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut update_instance: EventWriter<UpdateInstanceEvent>,
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
        if new_pose.is_changed() {
            update_instance.send(UpdateInstanceEvent {
                scenario: current_scenario_entity,
                instance: entity,
                update: UpdateInstance::Modify(new_pose.clone()),
            });
        }
    }
}

/// This system climbs up the scenario tree to retrieve inherited poses for a model instance, if any
fn retrieve_parent_pose(
    instance_entity: Entity,
    scenario_entity: Entity,
    children: &Query<&Children>,
    recall_instance: &Query<&RecallInstance>,
    scenarios: &Query<(Entity, &Affiliation<Entity>), With<ScenarioMarker>>,
    instance_modifiers: &Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
) -> Option<Pose> {
    let mut parent_pose: Option<Pose> = None;
    let mut entity = scenario_entity;
    while parent_pose.is_none() {
        let Ok((_, parent_scenario)) = scenarios.get(entity) else {
            break;
        };
        let Some((parent_entity, _)) = parent_scenario.0.and_then(|e| scenarios.get(e).ok()) else {
            break;
        };

        let instance_modifier_entities =
            get_instance_modifier_entities(parent_entity, children, instance_modifiers);

        if let Some(modifier_entity) = instance_modifier_entities.get(&instance_entity) {
            parent_pose = instance_modifiers
                .get(*modifier_entity)
                .ok()
                .and_then(|(i, _)| {
                    i.pose().or_else(|| {
                        recall_instance
                            .get(*modifier_entity)
                            .ok()
                            .and_then(|r| r.pose)
                    })
                });
        }
        entity = parent_entity;
    }
    parent_pose
}

/// This system searches for scenario children entities with the InstanceModifier component
/// and maps the affiliated model instance entity to the corresponding instance modifier entity
pub fn get_instance_modifier_entities(
    entity: Entity,
    children: &Query<&Children>,
    instance_modifiers: &Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
) -> HashMap<Entity, Entity> {
    let mut instance_to_modifier_entities = HashMap::<Entity, Entity>::new();
    if let Ok(scenario_children) = children.get(entity) {
        for child in scenario_children.iter() {
            if let Some(affiliated_entity) =
                instance_modifiers.get(*child).ok().and_then(|(_, a)| a.0)
            {
                instance_to_modifier_entities.insert(affiliated_entity, *child);
            }
        }
    };
    instance_to_modifier_entities
}

pub fn insert_new_instance_modifiers(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut instance_modifiers: Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
    added_instances: Query<(Entity, &Pose), (Added<InstanceMarker>, Without<Pending>)>,
    added_scenarios: Query<(Entity, &Affiliation<Entity>), Added<ScenarioMarker>>,
    children: Query<&Children>,
    current_scenario: Res<CurrentScenario>,
    scenarios: Query<Entity, With<ScenarioMarker>>,
) {
    let Some(current_scenario_entity) = current_scenario.0 else {
        return;
    };
    // Insert instance modifier entities when new scenarios are created
    for (scenario_entity, parent_scenario) in added_scenarios.iter() {
        if let Some(parent_scenario_entity) = parent_scenario.0 {
            // Inherit any instance modifiers from the parent scenario
            if let Ok(children) = children.get(parent_scenario_entity) {
                children.iter().for_each(|e| {
                    if let Ok((instance_modifier, affiliation)) =
                        instance_modifiers.get(*e).map(|(i, a)| match i {
                            InstanceModifier::Added(_) | InstanceModifier::Inherited(_) => {
                                (InstanceModifier::new_inherited(None), a.clone())
                            }
                            InstanceModifier::Hidden => (i.clone(), a.clone()),
                        })
                    {
                        commands
                            .spawn(instance_modifier)
                            .insert(affiliation)
                            .set_parent(scenario_entity);
                    }
                });
            }
        }
    }

    // Insert instance modifier entities when new model instances are spawned and placed
    if added_instances.is_empty() {
        if !added_scenarios.is_empty() {
            change_current_scenario.send(ChangeCurrentScenario(current_scenario_entity));
        }
        return;
    }
    let instance_modifier_entities =
        get_instance_modifier_entities(current_scenario_entity, &children, &instance_modifiers);

    for (instance_entity, new_pose) in added_instances.iter() {
        if let Some((mut instance_modifier, _)) = instance_modifier_entities
            .get(&instance_entity)
            .and_then(|modifier_entity| instance_modifiers.get_mut(*modifier_entity).ok())
        {
            // If an instance modifier entity already exists for this scenario, update it
            let instance_modifier = instance_modifier.as_mut();
            match instance_modifier {
                InstanceModifier::Added(_) => {
                    *instance_modifier = InstanceModifier::new_added(new_pose.clone())
                }
                InstanceModifier::Inherited(_) => {
                    *instance_modifier = InstanceModifier::new_inherited(Some(new_pose.clone()))
                }
                _ => {}
            }
        } else {
            // If instance modifier entity does not exist in this scenario, spawn one
            commands
                .spawn(InstanceModifier::new_added(new_pose.clone()))
                .insert(Affiliation(Some(instance_entity)))
                .set_parent(current_scenario_entity);
        }

        // Insert this new instance modifier into children scenarios as Inherited
        let mut subtree_dependents = HashSet::<Entity>::new();
        let mut queue = vec![current_scenario_entity];
        while let Some(scenario_entity) = queue.pop() {
            if let Ok(children) = children.get(scenario_entity) {
                children.iter().for_each(|e| {
                    subtree_dependents.insert(*e);
                    queue.push(*e);
                });
            }
        }
        // Only insert new instance modifier in children scenarios. Other parent/root
        // scenarios will not have access to this model instance
        for dependent in subtree_dependents.drain() {
            if let Ok(child_entity) = scenarios.get(dependent) {
                let child_instance_modifier_entities =
                    get_instance_modifier_entities(child_entity, &children, &instance_modifiers);
                if !child_instance_modifier_entities.contains_key(&instance_entity) {
                    // If instance modifier entity does not exist in this child scenario, spawn one
                    // Do nothing if it already exists, as it may be modified
                    commands
                        .spawn(InstanceModifier::new_inherited(None))
                        .insert(Affiliation(Some(instance_entity)))
                        .set_parent(child_entity);
                }
            }
        }
    }
    change_current_scenario.send(ChangeCurrentScenario(current_scenario_entity));
}

/// Checks that the current scenario's included instances are categorized correctly
pub fn handle_instance_updates(
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut instance_modifiers: Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
    mut update_instance: EventReader<UpdateInstanceEvent>,
    children: Query<&Children>,
    current_scenario: Res<CurrentScenario>,
    scenarios: Query<(Entity, &Affiliation<Entity>), With<ScenarioMarker>>,
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
            &instance_modifiers,
        );
        let instance_modifier_entities =
            get_instance_modifier_entities(update.scenario, &children, &instance_modifiers);

        if let Some(((mut instance_modifier, _), modifier_entity)) = instance_modifier_entities
            .get(&update.instance)
            .and_then(|modifier_entity| {
                instance_modifiers
                    .get_mut(*modifier_entity)
                    .ok()
                    .zip(Some(modifier_entity))
            })
        {
            let instance_modifier = instance_modifier.as_mut();

            match update.update {
                UpdateInstance::Include => {
                    let instance_pose = instance_modifier.pose().or(recall_instance
                        .get(*modifier_entity)
                        .ok()
                        .and_then(|r| r.pose));

                    if parent_pose.is_some() {
                        *instance_modifier = InstanceModifier::new_inherited(instance_pose)
                    } else if let Some(instance_pose) = instance_pose {
                        *instance_modifier = InstanceModifier::new_added(instance_pose);
                    } else {
                        let instance_id = model_instances
                            .get(update.instance)
                            .map(|n| n.0.clone())
                            .unwrap_or_else(|_| format!("{}", update.instance.index()));
                        error!(
                            "Unable to retrieve pose for instance {:?}, \
                                setting to default pose as AddedInstance in current scenario",
                            instance_id
                        );
                        *instance_modifier = InstanceModifier::new_added(Pose::default());
                    }
                }
                UpdateInstance::Hide => {
                    *instance_modifier = InstanceModifier::new_hidden();
                }
                UpdateInstance::Modify(new_pose) => {
                    // Update Pose changes
                    match instance_modifier {
                        InstanceModifier::Added(_) => {
                            *instance_modifier = InstanceModifier::new_added(new_pose.clone())
                        }
                        InstanceModifier::Inherited(_) => {
                            *instance_modifier =
                                InstanceModifier::new_inherited(Some(new_pose.clone()))
                        }
                        _ => {}
                    }
                }
                UpdateInstance::ResetPose => {
                    if parent_pose.is_some() {
                        *instance_modifier = InstanceModifier::new_inherited(None)
                    }
                }
            }
            if current_scenario.0.is_some_and(|e| e == update.scenario) {
                change_current_scenario.send(ChangeCurrentScenario(update.scenario));
            };
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
    mut create_new_scenario: EventWriter<CreateScenario>,
    mut delete: EventWriter<Delete>,
    mut scenarios: Query<
        (Entity, &Affiliation<Entity>, Option<&mut Dependents>),
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
        if let Some(parent_scenario_entity) =
            scenarios.get(request.0).map(|(_, a, _)| a.0).ok().flatten()
        {
            change_current_scenario.send(ChangeCurrentScenario(parent_scenario_entity));
        } else if let Some((root_scenario_entity, _, _)) = scenarios
            .iter()
            .filter(|(e, a, _)| request.0 != *e && a.0.is_none())
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
    scenarios: Query<(Entity, &NameInSite, &Affiliation<Entity>), With<ScenarioMarker>>,
    instance_modifiers: Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
) {
    for root in validate_events.read() {
        for (instance_entity, instance_name, _) in instances.iter() {
            if count_scenarios(&scenarios, instance_entity, &children, &instance_modifiers) > 0 {
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
    current_workspace: Res<CurrentWorkspace>,
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
        } else if let Some(site_entity) = current_workspace.root {
            cmd.set_parent(site_entity);
        }

        change_current_scenario.send(ChangeCurrentScenario(scenario_entity));
    }
}
