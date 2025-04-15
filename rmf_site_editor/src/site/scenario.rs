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
        Affiliation, CurrentScenario, Delete, Dependents, Group, InheritedInstance, InstanceMarker,
        InstanceModifier, IssueKey, ModelMarker, NameInSite, Pending, Pose, RecallInstance,
        ScenarioBundle, ScenarioMarker,
    },
    widgets::view_model_instances::count_scenarios,
    CurrentWorkspace, Issue, ValidateWorkspace,
};
use bevy::{prelude::*, utils::Uuid};
use std::collections::HashMap;

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
    ResetVisibility,
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
                    *visibility = if inherited.explicit_inclusion {
                        Visibility::Inherited
                    } else {
                        if let Some(v) = retrieve_parent_visibility(
                            entity,
                            *scenario_entity,
                            &children,
                            &scenarios,
                            &instance_modifiers,
                        ) {
                            if v {
                                Visibility::Inherited
                            } else {
                                Visibility::Hidden
                            }
                        } else {
                            error!(
                                "Instance {:?} is included in the current scenario, but no visibility found! \
                                Setting instance to included in current scenario.",
                                name.0
                            );
                            Visibility::Inherited
                        }
                    };
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
        let Some(parent_entity) = scenarios.get(entity).ok().and_then(|(_, a)| a.0) else {
            break;
        };

        if let Some(modifier_entity) =
            find_modifier_for_instance(instance_entity, parent_entity, children, instance_modifiers)
        {
            parent_pose = instance_modifiers
                .get(modifier_entity)
                .ok()
                .and_then(|(i, _)| {
                    i.pose().or_else(|| {
                        recall_instance
                            .get(modifier_entity)
                            .ok()
                            .and_then(|r| r.pose)
                    })
                });
        }
        entity = parent_entity;
    }
    parent_pose
}

fn retrieve_parent_visibility(
    instance_entity: Entity,
    scenario_entity: Entity,
    children: &Query<&Children>,
    scenarios: &Query<(Entity, &Affiliation<Entity>), With<ScenarioMarker>>,
    instance_modifiers: &Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
) -> Option<bool> {
    let mut parent_visibility: Option<bool> = None;
    let mut entity = scenario_entity;
    while parent_visibility.is_none() {
        let Some(parent_entity) = scenarios.get(entity).ok().and_then(|(_, a)| a.0) else {
            break;
        };

        if let Some(modifier_entity) =
            find_modifier_for_instance(instance_entity, parent_entity, children, instance_modifiers)
        {
            parent_visibility = instance_modifiers
                .get(modifier_entity)
                .ok()
                .and_then(|(i, _)| i.visibility());
        }
        entity = parent_entity;
    }
    parent_visibility
}

/// This system searches for the InstanceModifier affiliated with a specific model instance if any
pub fn find_modifier_for_instance(
    instance: Entity,
    scenario: Entity,
    children: &Query<&Children>,
    instance_modifiers: &Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
) -> Option<Entity> {
    if let Ok(scenario_children) = children.get(scenario) {
        for child in scenario_children.iter() {
            if instance_modifiers
                .get(*child)
                .is_ok_and(|(_, a)| a.0.is_some_and(|e| e == instance))
            {
                return Some(*child);
            }
        }
    };
    None
}

/// This system searches for scenario children entities with the InstanceModifier component
/// and maps the affiliated model instance entity to the corresponding instance modifier entity
pub fn get_instance_modifier_entities(
    scenario: Entity,
    children: &Query<&Children>,
    instance_modifiers: &Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
) -> HashMap<Entity, Entity> {
    let mut instance_to_modifier_entities = HashMap::<Entity, Entity>::new();
    if let Ok(scenario_children) = children.get(scenario) {
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

pub fn manage_instance_modifiers(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut delete: EventWriter<Delete>,
    mut instance_modifiers: Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
    mut removals: RemovedComponents<Pose>,
    children: Query<&Children>,
    current_scenario: Res<CurrentScenario>,
    model_instances: Query<(Entity, Ref<Pose>), Without<Pending>>,
    scenarios: Query<(Entity, Ref<Affiliation<Entity>>), With<ScenarioMarker>>,
) {
    let Some(current_scenario_entity) = current_scenario.0 else {
        return;
    };
    // Insert instance modifier entities when new scenarios are created
    for (scenario_entity, parent_scenario) in scenarios.iter() {
        if parent_scenario.is_added() {
            if let Some(parent_scenario_entity) = parent_scenario.0 {
                // Inherit any instance modifiers from the parent scenario
                if let Ok(children) = children.get(parent_scenario_entity) {
                    children.iter().for_each(|e| {
                        if let Ok((instance_modifier, affiliation)) = instance_modifiers
                            .get(*e)
                            .map(|(_, a)| (InstanceModifier::inherited(), a.clone()))
                        {
                            commands
                                .spawn(instance_modifier)
                                .insert(affiliation)
                                .set_parent(scenario_entity);
                        }
                    });
                }
            } else {
                // If root scenario, mark all instance modifiers as Hidden
                for (instance_entity, _) in model_instances.iter() {
                    commands
                        .spawn(InstanceModifier::Hidden)
                        .insert(Affiliation(Some(instance_entity)))
                        .set_parent(scenario_entity);
                }
            }
            change_current_scenario.send(ChangeCurrentScenario(scenario_entity));
        }
    }

    // Insert instance modifier entities when new model instances are spawned and placed
    for (instance_entity, instance_pose) in model_instances.iter() {
        if instance_pose.is_added() {
            if let Some((mut instance_modifier, _)) = find_modifier_for_instance(
                instance_entity,
                current_scenario_entity,
                &children,
                &instance_modifiers,
            )
            .and_then(|modifier_entity| instance_modifiers.get_mut(modifier_entity).ok())
            {
                // If an instance modifier entity already exists for this scenario, update it
                let instance_modifier = instance_modifier.as_mut();
                match instance_modifier {
                    InstanceModifier::Added(_) => {
                        *instance_modifier = InstanceModifier::added(instance_pose.clone())
                    }
                    InstanceModifier::Inherited(inherited) => {
                        inherited.modified_pose = Some(instance_pose.clone())
                    }
                    InstanceModifier::Hidden => {}
                }
            } else {
                // If instance modifier entity does not exist in this scenario, spawn one
                commands
                    .spawn(InstanceModifier::added(instance_pose.clone()))
                    .insert(Affiliation(Some(instance_entity)))
                    .set_parent(current_scenario_entity);
            }

            // Insert instance modifier into remaining scenarios
            for (scenario_entity, parent_scenario) in scenarios.iter() {
                if scenario_entity == current_scenario_entity {
                    continue;
                }

                // Crawl up scenario tree to check if this is a descendent of the current scenario
                let mut parent_entity: Option<Entity> = parent_scenario.0.clone();
                while parent_entity.is_some() {
                    if parent_entity.is_some_and(|e| e == current_scenario_entity) {
                        break;
                    }
                    parent_entity = parent_entity
                        .and_then(|e| scenarios.get(e).ok())
                        .and_then(|(_, a)| a.0);
                }

                // If instance modifier entity does not exist in this child scenario, spawn one
                // Do nothing if it already exists, as it may be modified
                if find_modifier_for_instance(
                    instance_entity,
                    scenario_entity,
                    &children,
                    &instance_modifiers,
                )
                .is_none()
                {
                    if parent_entity.is_some_and(|e| e == current_scenario_entity) {
                        // Insert this new instance modifier into children scenarios as Inherited
                        commands
                            .spawn(InstanceModifier::inherited())
                            .insert(Affiliation(Some(instance_entity)))
                            .set_parent(scenario_entity);
                    } else {
                        // Insert this new instance modifier into other scenarios as Hidden
                        commands
                            .spawn(InstanceModifier::Hidden)
                            .insert(Affiliation(Some(instance_entity)))
                            .set_parent(scenario_entity);
                    }
                }
            }
        }
    }

    // Check for modifiers affiliated with deleted Instance or missing valid affiliations.
    if !removals.is_empty() {
        for instance_entity in removals.read() {
            for (scenario_entity, _) in scenarios.iter() {
                if let Some(modifier_entity) = find_modifier_for_instance(
                    instance_entity,
                    scenario_entity,
                    &children,
                    &instance_modifiers,
                ) {
                    // Instance modifier is affiliated to a non-existing instance
                    delete.send(Delete::new(modifier_entity));
                }
            }
        }
    }
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
        if let Some(((mut instance_modifier, _), modifier_entity)) = find_modifier_for_instance(
            update.instance,
            update.scenario,
            &children,
            &instance_modifiers,
        )
        .and_then(|modifier_entity| {
            instance_modifiers
                .get_mut(modifier_entity)
                .ok()
                .zip(Some(modifier_entity))
        }) {
            let instance_modifier = instance_modifier.as_mut();
            let has_parent = scenarios
                .get(update.scenario)
                .is_ok_and(|(_, a)| a.0.is_some());

            match update.update {
                UpdateInstance::Include => {
                    let recall_modifier = recall_instance.get(modifier_entity).ok();
                    let instance_pose = instance_modifier
                        .pose()
                        .or(recall_modifier.and_then(|r| r.pose));
                    if has_parent
                        && recall_modifier.is_some_and(|m| match m.modifier {
                            Some(InstanceModifier::Inherited(_)) => true,
                            _ => false,
                        })
                    {
                        *instance_modifier = InstanceModifier::Inherited(InheritedInstance {
                            modified_pose: instance_pose,
                            explicit_inclusion: true,
                        });
                    } else if let Some(instance_pose) = instance_pose {
                        *instance_modifier = InstanceModifier::added(instance_pose);
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
                        *instance_modifier = InstanceModifier::added(Pose::default());
                    }
                }
                UpdateInstance::Hide => {
                    *instance_modifier = InstanceModifier::Hidden;
                }
                UpdateInstance::Modify(new_pose) => {
                    // Update Pose changes
                    match instance_modifier {
                        InstanceModifier::Added(_) => {
                            *instance_modifier = InstanceModifier::added(new_pose.clone())
                        }
                        InstanceModifier::Inherited(inherited) => {
                            inherited.modified_pose = Some(new_pose.clone())
                        }
                        InstanceModifier::Hidden => {}
                    }
                }
                UpdateInstance::ResetPose => {
                    if has_parent {
                        match instance_modifier {
                            InstanceModifier::Inherited(inherited) => {
                                inherited.modified_pose = None
                            }
                            _ => {}
                        }
                    }
                }
                UpdateInstance::ResetVisibility => {
                    if has_parent {
                        match instance_modifier {
                            InstanceModifier::Inherited(inherited) => {
                                inherited.explicit_inclusion = false
                            }
                            _ => {}
                        }
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
    current_workspace: Res<CurrentWorkspace>,
) {
    for new in new_scenarios.read() {
        let mut cmd = commands.spawn(ScenarioBundle::<Entity>::new(
            new.name.clone(),
            new.parent.clone(),
        ));

        if let Some(parent) = current_workspace.root {
            cmd.set_parent(parent);
        } else {
            error!("Missing workspace for a new root scenario!");
        }
    }
}
