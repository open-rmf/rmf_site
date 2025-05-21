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
        InstanceModifier, IssueKey, ModelMarker, NameInSite, Pending, PendingModel, Pose,
        RecallInstance, ScenarioBundle, ScenarioMarker, ScenarioModifiers,
    },
    widgets::view_model_instances::count_scenarios,
    CurrentWorkspace, Issue, ValidateWorkspace,
};
use bevy::ecs::{hierarchy::ChildOf, system::SystemParam};
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

#[derive(SystemParam)]
pub struct GetModifier<'w, 's, T: Component + Clone + Default> {
    pub scenarios: Query<
        'w,
        's,
        (
            &'static ScenarioModifiers<Entity>,
            &'static Affiliation<Entity>,
        ),
        With<ScenarioMarker>,
    >,
    pub modifiers: Query<'w, 's, &'static T>,
}

impl<'w, 's, T: Component + Clone + Default> GetModifier<'w, 's, T> {
    pub fn get(&self, scenario: Entity, entity: Entity) -> Option<&T> {
        let mut modifier: Option<&T> = None;
        let mut scenario_entity = scenario;
        while modifier.is_none() {
            let Ok((scenario_modifiers, scenario_parent)) = self.scenarios.get(scenario_entity)
            else {
                break;
            };
            if let Some(target_modifier) = scenario_modifiers
                .get(&entity)
                .and_then(|e| self.modifiers.get(*e).ok())
            {
                modifier = Some(target_modifier);
                break;
            }

            if let Some(parent_entity) = scenario_parent.0 {
                scenario_entity = parent_entity;
            } else {
                // Modifier does not exist in the current scenario tree
                break;
            }
        }
        modifier
    }
}

#[derive(Clone, Debug, Event)]
pub struct AddModifier {
    instance: Entity, // TODO(@xiyuoh) Change this field to something else so that we can use this across all types of scenario modifiers
    modifier: Entity,
    scenario: Entity,
    to_root: bool,
}

impl AddModifier {
    pub fn new(instance: Entity, modifier: Entity, scenario: Entity) -> Self {
        Self {
            instance,
            modifier,
            scenario,
            to_root: false,
        }
    }

    pub fn new_to_root(instance: Entity, modifier: Entity, scenario: Entity) -> Self {
        Self {
            instance,
            modifier,
            scenario,
            to_root: true,
        }
    }
}

#[derive(Clone, Debug, Event)]
pub struct RemoveModifier {
    instance: Entity, // TODO(@xiyuoh) Change this field to something else so that we can use this across all types of scenario modifiers
    scenario: Entity,
}

impl RemoveModifier {
    pub fn new(instance: Entity, scenario: Entity) -> Self {
        Self { instance, scenario }
    }
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
    mut commands: Commands,
    mut add_modifier: EventWriter<AddModifier>,
    mut select: EventWriter<Select>,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    mut instances: Query<
        (Entity, &mut Pose, &mut Visibility),
        (With<InstanceMarker>, Without<PendingModel>),
    >,
    get_modifier: GetModifier<InstanceModifier>,
    recall_instance: Query<&RecallInstance>,
    selection: Res<Selection>,
) {
    if let Some(ChangeCurrentScenario(scenario_entity)) = change_current_scenario.read().last() {
        let mut deselect = false;

        // Loop over every model instance in this site
        for (instance_entity, mut pose, mut visibility) in instances.iter_mut() {
            let Some(instance_modifier) = get_modifier.get(*scenario_entity, instance_entity)
            else {
                // No instance modifier exists for this model instance/scenario pairing
                // TODO(@xiyuoh) catch this with a diagnostic
                // Make sure that an instance modifier exists in the current scenario tree
                let root_modifier_entity = commands.spawn(InstanceModifier::Hidden).id();
                add_modifier.write(AddModifier::new_to_root(
                    instance_entity,
                    root_modifier_entity,
                    *scenario_entity,
                ));
                continue;
            };

            let fallback_pose = Pose::default(); // TODO(@xiyuoh) retrieve fallback pose
            let fallback_visibility = Visibility::Hidden;

            *pose = instance_modifier
                .pose()
                .or_else(|| {
                    retrieve_parent_pose(
                        instance_entity,
                        *scenario_entity,
                        &get_modifier,
                        &recall_instance,
                    )
                })
                .unwrap_or(fallback_pose);

            *visibility = instance_modifier
                .visibility()
                .or_else(|| {
                    retrieve_parent_visibility(instance_entity, *scenario_entity, &get_modifier)
                })
                .map(|v| {
                    if v {
                        Visibility::Inherited
                    } else {
                        Visibility::Hidden
                    }
                })
                .unwrap_or(fallback_visibility);

            // TODO(@xiyuoh) Consider if we want to deselect hidden instances
            if *visibility == Visibility::Hidden
                && selection.0.is_some_and(|e| e == instance_entity)
            {
                deselect = true;
            }
        }

        if deselect {
            select.write(Select::new(None));
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
            update_instance.write(UpdateInstanceEvent {
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
    get_modifier: &GetModifier<InstanceModifier>,
    recall_instance: &Query<&RecallInstance>,
) -> Option<Pose> {
    let mut parent_pose: Option<Pose> = None;
    let mut entity = scenario_entity;
    while parent_pose.is_none() {
        let Some(parent_entity) = get_modifier
            .scenarios
            .get(entity)
            .ok()
            .and_then(|(_, p)| p.0)
        else {
            break;
        };

        let Ok((parent_scenario_modifiers, _)) = get_modifier.scenarios.get(parent_entity) else {
            break;
        };
        if let Some(instance_modifier) = get_modifier.get(parent_entity, instance_entity) {
            parent_pose = instance_modifier.pose().or_else(|| {
                parent_scenario_modifiers
                    .get(&instance_entity)
                    .and_then(|e| recall_instance.get(*e).ok())
                    .and_then(|r| r.pose)
            })
        }
        entity = parent_entity;
    }
    parent_pose
}

/// This system climbs up the scenario tree to retrieve inherited visibility for a model instance, if any
pub fn retrieve_parent_visibility(
    instance_entity: Entity,
    scenario_entity: Entity,
    get_modifier: &GetModifier<InstanceModifier>,
) -> Option<bool> {
    let mut parent_visibility: Option<bool> = None;
    let mut entity = scenario_entity;
    while parent_visibility.is_none() {
        let Some(parent_entity) = get_modifier
            .scenarios
            .get(entity)
            .ok()
            .and_then(|(_, p)| p.0)
        else {
            break;
        };

        if let Some(instance_modifier) = get_modifier.get(parent_entity, instance_entity) {
            parent_visibility = instance_modifier.visibility();
        }
        entity = parent_entity;
    }
    parent_visibility
}

pub fn handle_scenario_modifiers(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut add_modifier: EventReader<AddModifier>,
    mut remove_modifier: EventReader<RemoveModifier>,
    mut scenarios: Query<
        (&mut ScenarioModifiers<Entity>, &Affiliation<Entity>),
        With<ScenarioMarker>,
    >,
    current_scenario: Res<CurrentScenario>,
) {
    for remove in remove_modifier.read() {
        let Ok((mut scenario_modifiers, _)) = scenarios.get_mut(remove.scenario) else {
            continue;
        };
        if let Some(modifier) = scenario_modifiers.remove(&remove.instance) {
            commands.entity(modifier).despawn();
        }

        if current_scenario.0.is_some_and(|e| e == remove.scenario) {
            change_current_scenario.write(ChangeCurrentScenario(remove.scenario));
        };
    }

    for add in add_modifier.read() {
        let scenario_entity = if add.to_root {
            let mut target_scenario = add.scenario;
            let mut root_scenario: Option<Entity> = None;
            while root_scenario.is_none() {
                let Ok((_, parent_scenario)) = scenarios.get(target_scenario) else {
                    break;
                };
                if let Some(parent_entity) = parent_scenario.0 {
                    target_scenario = parent_entity;
                } else {
                    root_scenario = Some(target_scenario);
                    break;
                }
            }
            if let Some(root_scenario_entity) = root_scenario {
                root_scenario_entity
            } else {
                error!("No root scenario found for the current scenario tree!");
                continue;
            }
        } else {
            add.scenario
        };

        let Ok((mut scenario_modifiers, _)) = scenarios.get_mut(scenario_entity) else {
            continue;
        };
        // If a modifier entity already exists, despawn
        if let Some(current_modifier) = scenario_modifiers.remove(&add.instance) {
            commands.entity(current_modifier).despawn();
        }

        commands
            .entity(add.modifier)
            .insert(Affiliation(Some(add.instance)))
            .insert(ChildOf(scenario_entity));
        scenario_modifiers.insert(add.instance, add.modifier);

        if current_scenario.0.is_some_and(|e| e == add.scenario) {
            change_current_scenario.write(ChangeCurrentScenario(add.scenario));
        };
    }
}

pub fn insert_new_instance_modifiers(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut add_modifier: EventWriter<AddModifier>,
    mut instance_modifiers: Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
    current_scenario: Res<CurrentScenario>,
    model_instances: Query<(Entity, Ref<Pose>), Without<Pending>>,
    scenarios: Query<
        (
            Entity,
            &mut ScenarioModifiers<Entity>,
            Ref<Affiliation<Entity>>,
        ),
        With<ScenarioMarker>,
    >,
) {
    let Some(current_scenario_entity) = current_scenario.0 else {
        return;
    };
    // Insert instance modifier entities when new scenarios are created
    for (scenario_entity, _, parent_scenario) in scenarios.iter() {
        if parent_scenario.is_added() {
            // If root scenario, mark all instance modifiers as Hidden
            if parent_scenario.0.is_none() {
                for (instance_entity, _) in model_instances.iter() {
                    let modifier_entity = commands.spawn(InstanceModifier::Hidden).id();
                    add_modifier.write(AddModifier::new(
                        instance_entity,
                        modifier_entity,
                        scenario_entity,
                    ));
                }
            }
            change_current_scenario.write(ChangeCurrentScenario(scenario_entity));
        }
    }

    // Insert instance modifier entities when new model instances are spawned and placed
    for (instance_entity, instance_pose) in model_instances.iter() {
        if instance_pose.is_added() {
            let Ok((_, current_scenario_modifiers, _)) = scenarios.get(current_scenario_entity)
            else {
                continue;
            };
            if let Some((mut instance_modifier, _)) = current_scenario_modifiers
                .get(&instance_entity)
                .and_then(|e| instance_modifiers.get_mut(*e).ok())
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
                let modifier_entity = commands
                    .spawn(InstanceModifier::added(instance_pose.clone()))
                    .id();
                add_modifier.write(AddModifier::new(
                    instance_entity,
                    modifier_entity,
                    current_scenario_entity,
                ));
            }

            // Retrieve root scenario of current scenario
            let mut current_root_entity: Entity = current_scenario_entity;
            while let Ok((_, _, parent_scenario)) = scenarios.get(current_root_entity) {
                if let Some(parent_scenario_entity) = parent_scenario.0 {
                    current_root_entity = parent_scenario_entity;
                } else {
                    break;
                }
            }

            // Insert instance modifier into all root scenarios outside of the current tree as hidden
            for (scenario_entity, _, parent_scenario) in scenarios.iter() {
                if parent_scenario.0.is_some() || scenario_entity == current_root_entity {
                    continue;
                }
                let modifier_entity = commands.spawn(InstanceModifier::Hidden).id();
                add_modifier.write(AddModifier::new(
                    instance_entity,
                    modifier_entity,
                    scenario_entity,
                ));
            }
        }
    }
}

/// Checks that the current scenario's included instances are categorized correctly
pub fn handle_instance_updates(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut add_modifier: EventWriter<AddModifier>,
    mut instance_modifiers: Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
    mut remove_modifier: EventWriter<RemoveModifier>,
    mut update_instance: EventReader<UpdateInstanceEvent>,
    current_scenario: Res<CurrentScenario>,
    recall_instance: Query<&RecallInstance>,
    scenarios: Query<(&mut ScenarioModifiers<Entity>, &Affiliation<Entity>), With<ScenarioMarker>>,
) {
    for update in update_instance.read() {
        let Ok((scenario_modifiers, _)) = scenarios.get(update.scenario) else {
            continue;
        };

        if let Some((mut instance_modifier, modifier_entity)) =
            scenario_modifiers.get(&update.instance).and_then(|e| {
                instance_modifiers
                    .get_mut(*e)
                    .ok()
                    .map(|(m, _)| m)
                    .zip(Some(e))
            })
        {
            let instance_modifier = instance_modifier.as_mut();
            match update.update {
                UpdateInstance::Include => {
                    match instance_modifier {
                        InstanceModifier::Added(_) => continue,
                        InstanceModifier::Inherited(inherited) => {
                            inherited.explicit_inclusion = true;
                        }
                        InstanceModifier::Hidden => {
                            if let Some(recall_modifier) = recall_instance
                                .get(*modifier_entity)
                                .ok()
                                .and_then(|r| r.modifier.clone())
                            {
                                match recall_modifier {
                                    InstanceModifier::Added(_) => {
                                        // TODO(@xiyuoh) setup method to retrieve fallback pose and insert value here
                                        let fallback_pose = Pose::default();
                                        *instance_modifier = InstanceModifier::added(
                                            recall_modifier.pose().unwrap_or(fallback_pose),
                                        );
                                    }
                                    InstanceModifier::Inherited(_) => {
                                        *instance_modifier =
                                            InstanceModifier::Inherited(InheritedInstance {
                                                modified_pose: recall_modifier.pose(),
                                                explicit_inclusion: true,
                                            });
                                    }
                                    InstanceModifier::Hidden => {} // We don't recall Hidden modifiers
                                }
                            } else {
                                let modifier_entity = commands
                                    .spawn(InstanceModifier::inherited_with_inclusion())
                                    .id();
                                add_modifier.write(AddModifier::new(
                                    update.instance,
                                    modifier_entity,
                                    update.scenario,
                                ));
                            }
                        }
                    }
                }
                UpdateInstance::Hide => {
                    *instance_modifier = InstanceModifier::Hidden;
                }
                UpdateInstance::Modify(new_pose) => match instance_modifier {
                    InstanceModifier::Added(_) => {
                        *instance_modifier = InstanceModifier::added(new_pose)
                    }
                    InstanceModifier::Inherited(inherited) => {
                        inherited.modified_pose = Some(new_pose)
                    }
                    InstanceModifier::Hidden => {}
                },
                UpdateInstance::ResetPose | UpdateInstance::ResetVisibility => {
                    let inherited = match instance_modifier {
                        InstanceModifier::Inherited(inherited) => inherited,
                        _ => continue,
                    };
                    match update.update {
                        UpdateInstance::ResetPose => inherited.modified_pose = None,
                        UpdateInstance::ResetVisibility => inherited.explicit_inclusion = false,
                        _ => continue,
                    }
                    if !inherited.modified() {
                        remove_modifier
                            .write(RemoveModifier::new(update.instance, update.scenario));
                    }
                }
            }
        } else {
            let instance_modifier = match update.update {
                UpdateInstance::Include => InstanceModifier::inherited_with_inclusion(),
                UpdateInstance::Hide => InstanceModifier::Hidden,
                UpdateInstance::Modify(new_pose) => InstanceModifier::inherited_with_pose(new_pose),
                UpdateInstance::ResetPose | UpdateInstance::ResetVisibility => {
                    continue;
                }
            };
            let modifier_entity = commands.spawn(instance_modifier).id();
            add_modifier.write(AddModifier::new(
                update.instance,
                modifier_entity,
                update.scenario,
            ));
        }

        if current_scenario.0.is_some_and(|e| e == update.scenario) {
            change_current_scenario.write(ChangeCurrentScenario(update.scenario));
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
        (Entity, &Affiliation<Entity>, Option<&mut Dependents>),
        With<ScenarioMarker>,
    >,
    children: Query<&Children>,
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

        // Change to parent scenario, else root, else create an empty scenario and switch to it
        if let Some(parent_scenario_entity) =
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
    mut update_instance: EventWriter<UpdateInstanceEvent>,
    mut validate_events: EventReader<ValidateWorkspace>,
    get_modifier: GetModifier<InstanceModifier>,
    instances: Query<
        (Entity, &NameInSite, &Affiliation<Entity>),
        (With<ModelMarker>, Without<Group>),
    >,
    scenarios: Query<
        (Entity, &ScenarioModifiers<Entity>, &Affiliation<Entity>),
        With<ScenarioMarker>,
    >,
) {
    for root in validate_events.read() {
        for (instance_entity, instance_name, _) in instances.iter() {
            if count_scenarios(
                &scenarios,
                instance_entity,
                &get_modifier,
                &mut update_instance,
            ) > 0
            {
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
        let mut cmd = commands.spawn((
            ScenarioBundle::<Entity>::new(new.name.clone(), new.parent.clone()),
            ScenarioModifiers::<Entity>::default(),
        ));

        if let Some(parent) = current_workspace.root {
            cmd.insert(ChildOf(parent));
        } else {
            error!("Missing workspace for a new root scenario!");
        }
    }
}
