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
        AddModifier, Affiliation, CurrentScenario, Delete, Dependents, GetModifier, Group,
        Inclusion, InstanceMarker, IssueKey, LastSetValue, ModelMarker, Modifier, NameInSite,
        Pending, PendingModel, Pose, Property, ScenarioBundle, ScenarioMarker, ScenarioModifiers,
        UpdateModifier, UpdateProperty,
    },
    CurrentWorkspace, Issue, ValidateWorkspace,
};
use bevy::ecs::{hierarchy::ChildOf, system::SystemState};
use bevy::prelude::*;
use rmf_site_picking::{Select, Selection};
use bevy::platform::collections::HashSet;
use uuid::Uuid;

#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentScenario(pub Entity);

#[derive(Clone, Debug, Event)]
pub struct CreateScenario {
    pub name: Option<String>,
    pub parent: Option<Entity>,
}

#[derive(Clone, Debug, Copy)]
pub enum UpdateInstance {
    Include,
    Hide,
    Modify(Pose),
    ResetPose,
    ResetVisibility,
}

impl Property for Pose {
    fn get_fallback(for_element: Entity, _in_scenario: Entity, world: &mut World) -> Pose {
        let mut state: SystemState<Query<&LastSetValue<Pose>>> = SystemState::new(world);
        let last_set_pose = state.get(world);

        last_set_pose
            .get(for_element)
            .map(|value| value.0)
            .unwrap_or(Pose::default())
    }

    fn insert(for_element: Entity, in_scenario: Entity, value: Pose, world: &mut World) {
        let mut modifier_state: SystemState<(
            Query<(&mut Modifier<Pose>, &Affiliation)>,
            Query<(Entity, &ScenarioModifiers, Ref<Affiliation>), With<ScenarioMarker>>,
        )> = SystemState::new(world);
        let (mut pose_modifiers, scenarios) = modifier_state.get_mut(world);

        // Insert instance pose modifier entities when new model instances are spawned and placed
        let Ok((_, scenario_modifiers, _)) = scenarios.get(in_scenario) else {
            return;
        };
        let mut new_pose_modifiers = Vec::<(Modifier<Pose>, Entity)>::new();
        let mut new_visibility_modifiers = Vec::<(Modifier<Visibility>, Entity)>::new();

        if let Some((mut pose_modifier, _)) = scenario_modifiers
            .get(&for_element)
            .and_then(|e| pose_modifiers.get_mut(*e).ok())
        {
            // If an instance pose modifier entity already exists for this scenario, update it
            **pose_modifier = value.clone();
        } else {
            // If pose modifier entity does not exist in this scenario, spawn one
            new_pose_modifiers.push((Modifier::<Pose>::new(value.clone()), in_scenario));
        }

        // Retrieve root scenario of current scenario
        let mut current_root_entity: Entity = in_scenario;
        while let Ok((_, _, parent_scenario)) = scenarios.get(current_root_entity) {
            if let Some(parent_scenario_entity) = parent_scenario.0 {
                current_root_entity = parent_scenario_entity;
            } else {
                break;
            }
        }
        // Insert visibility modifier into all root scenarios outside of the current tree as hidden
        for (scenario_entity, _, parent_scenario) in scenarios.iter() {
            if parent_scenario.0.is_some() || scenario_entity == current_root_entity {
                continue;
            }
            new_visibility_modifiers.push((
                Modifier::<Visibility>::new(Visibility::Hidden),
                scenario_entity,
            ));
        }

        // Spawn all new modifier entities
        let new_current_scenario_modifiers = new_pose_modifiers
            .iter()
            .map(|(modifier, scenario)| {
                (
                    world
                        .spawn(modifier.clone())
                        // Mark all newly spawned instances as visible
                        .insert(Modifier::<Visibility>::new(Visibility::Inherited))
                        .id(),
                    *scenario,
                )
            })
            .collect::<Vec<(Entity, Entity)>>();
        let mut new_modifier_entities = new_visibility_modifiers
            .iter()
            .map(|(modifier, scenario)| (world.spawn(modifier.clone()).id(), *scenario))
            .collect::<Vec<(Entity, Entity)>>();
        new_modifier_entities.extend(new_current_scenario_modifiers);

        let mut events_state: SystemState<EventWriter<AddModifier>> = SystemState::new(world);
        let mut add_modifier = events_state.get_mut(world);
        for (modifier_entity, scenario_entity) in new_modifier_entities.iter() {
            add_modifier.write(AddModifier::new(
                for_element,
                *modifier_entity,
                *scenario_entity,
            ));
        }
    }

    fn insert_on_new_scenario(_in_scenario: Entity, _world: &mut World) {
        // Do nothing when new root scenarios are created. When an instance is
        // toggled to be included and visible, a pose modifier will be inserted
        // from fallback pose values.
    }
}

impl Property for Visibility {
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Visibility {
        // We want the instance to be hidden by default, and only visible
        // when intentionally toggled
        Visibility::Hidden
    }

    fn insert(_for_element: Entity, _in_scenario: Entity, _value: Visibility, _world: &mut World) {
        // Do nothing when new Visibility components are inserted. Newly spawned
        // model instances are handled in Pose::insert()
    }

    fn insert_on_new_scenario(in_scenario: Entity, world: &mut World) {
        let mut instance_state: SystemState<(
            Query<&Children>,
            Query<(&Modifier<Visibility>, &Affiliation)>,
            Query<Entity, (With<InstanceMarker>, Without<Pending>)>,
        )> = SystemState::new(world);
        let (children, visibility_modifiers, model_instances) = instance_state.get_mut(world);

        // Spawn visibility modifier entities when new root scenarios are created
        let mut have_instance = HashSet::new();
        if let Ok(scenario_children) = children.get(in_scenario) {
            for child in scenario_children {
                if let Ok((_, a)) = visibility_modifiers.get(*child) {
                    if let Some(a) = a.0 {
                        have_instance.insert(a);
                    }
                }
            }
        }

        let mut target_instances = HashSet::new();
        for instance_entity in model_instances.iter() {
            if !have_instance.contains(&instance_entity) {
                target_instances.insert(instance_entity);
            }
        }

        let mut new_modifiers = Vec::<(Entity, Entity)>::new();
        for target in target_instances.iter() {
            // Mark all visibility modifiers as Hidden
            new_modifiers.push((
                *target,
                world
                    .commands()
                    .spawn(Modifier::<Visibility>::new(Visibility::Hidden))
                    .id(),
            ));
        }

        let mut events_state: SystemState<(
            EventWriter<AddModifier>,
            EventWriter<ChangeCurrentScenario>,
        )> = SystemState::new(world);
        let (mut add_modifier, mut change_current_scenario) = events_state.get_mut(world);
        for (instance_entity, modifier_entity) in new_modifiers.iter() {
            add_modifier.write(AddModifier::new(
                *instance_entity,
                *modifier_entity,
                in_scenario,
            ));
        }
        change_current_scenario.write(ChangeCurrentScenario(in_scenario));
    }
}

/// Handles updates when the current scenario has changed, and trigger property updates for scenario elements
pub fn update_current_scenario(
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    mut update_property: EventWriter<UpdateProperty>,
    instances: Query<Entity, (With<InstanceMarker>, Without<PendingModel>)>,
) {
    if let Some(ChangeCurrentScenario(scenario_entity)) = change_current_scenario.read().last() {
        *current_scenario = CurrentScenario(Some(*scenario_entity));
        for instance_entity in instances.iter() {
            info!("Found instance {:?}", instance_entity);
            update_property.write(UpdateProperty::new(instance_entity, *scenario_entity));
        }
    }
}

pub fn check_selected_is_visible(
    mut select: EventWriter<Select>,
    selection: Res<Selection>,
    visibility: Query<&Visibility>,
) {
    if selection.0.is_some_and(|e| {
        visibility.get(e).is_ok_and(|v| match v {
            Visibility::Hidden => true,
            _ => false,
        })
    }) {
        select.write(Select::new(None));
    }
}

/// Tracks pose changes for instances in the current scenario to update its properties
pub fn update_model_instance_poses(
    current_scenario: Res<CurrentScenario>,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut update_instance: EventWriter<UpdateModifier<UpdateInstance>>,
    changed_instances: Query<(Entity, Ref<Pose>), (With<InstanceMarker>, Without<Pending>)>,
    changed_last_set_pose: Query<(), Changed<LastSetValue<Pose>>>,
) {
    // Do nothing if scenario has changed, as we rely on pose changes by the user and not the system updating instances
    for ChangeCurrentScenario(_) in change_current_scenario.read() {
        return;
    }
    let Some(current_scenario_entity) = current_scenario.0 else {
        return;
    };

    for (entity, new_pose) in changed_instances.iter() {
        if new_pose.is_changed()
            && !new_pose.is_added()
            && changed_last_set_pose.get(entity).is_err()
        {
            // Only mark an instance as modified if its pose changed due to user
            // interaction, not because it was updated by scenarios
            update_instance.write(UpdateModifier::new(
                current_scenario_entity,
                entity,
                UpdateInstance::Modify(new_pose.clone()),
            ));
        }
    }
}

/// Handles updates to model instance modifiers for all scenarios
pub fn handle_instance_modifier_updates(
    mut commands: Commands,
    mut add_modifier: EventWriter<AddModifier>,
    mut update_instance: EventReader<UpdateModifier<UpdateInstance>>,
    mut update_property: EventWriter<UpdateProperty>,
    mut pose_modifiers: Query<&mut Modifier<Pose>, With<Affiliation>>,
    mut visibility_modifiers: Query<&mut Modifier<Visibility>, With<Affiliation>>,
    scenarios: Query<(&ScenarioModifiers, &Affiliation), With<ScenarioMarker>>,
) {
    for update in update_instance.read() {
        let Ok((scenario_modifiers, parent_scenario)) = scenarios.get(update.scenario) else {
            continue;
        };

        let modifier_entity = scenario_modifiers.get(&update.element);
        let pose_modifier = modifier_entity.and_then(|e| pose_modifiers.get_mut(*e).ok());
        let visibility_modifier =
            modifier_entity.and_then(|e| visibility_modifiers.get_mut(*e).ok());

        match update.update {
            UpdateInstance::Include | UpdateInstance::Hide => {
                let new_visibility = match update.update {
                    UpdateInstance::Include => Visibility::Inherited,
                    UpdateInstance::Hide => Visibility::Hidden,
                    _ => continue,
                };
                if let Some(mut visibility_modifier) = visibility_modifier {
                    **visibility_modifier = new_visibility;
                } else if let Some(modifier_entity) = modifier_entity {
                    commands
                        .entity(*modifier_entity)
                        .insert(Modifier::<Visibility>::new(new_visibility));
                } else {
                    let modifier_entity = commands
                        .spawn(Modifier::<Visibility>::new(new_visibility))
                        .id();
                    add_modifier.write(AddModifier::new(
                        update.element,
                        modifier_entity,
                        update.scenario,
                    ));
                }
            }
            UpdateInstance::Modify(new_pose) => {
                if let Some(mut pose_modifier) = pose_modifier {
                    **pose_modifier = new_pose.clone();
                    commands
                        .entity(update.element)
                        .insert(LastSetValue::<Pose>::new(new_pose));
                    // Do not trigger PropertyPlugin<Pose> if pose for existing modifier
                    // was modified by user
                    continue;
                } else if let Some(modifier_entity) = modifier_entity {
                    commands
                        .entity(*modifier_entity)
                        .insert(Modifier::<Pose>::new(new_pose));
                } else {
                    let modifier_entity = commands.spawn(Modifier::<Pose>::new(new_pose)).id();
                    add_modifier.write(AddModifier::new(
                        update.element,
                        modifier_entity,
                        update.scenario,
                    ));
                }
            }
            UpdateInstance::ResetPose | UpdateInstance::ResetVisibility => {
                // Only process resets if this is not a root scenario
                if parent_scenario.0.is_some() {
                    if let Some(modifier_entity) = modifier_entity {
                        match update.update {
                            UpdateInstance::ResetPose => {
                                commands.entity(*modifier_entity).remove::<Modifier<Pose>>();
                            }
                            UpdateInstance::ResetVisibility => {
                                commands
                                    .entity(*modifier_entity)
                                    .remove::<Modifier<Visibility>>();
                            }
                            _ => continue,
                        }
                    }
                }
            }
        }

        update_property.write(UpdateProperty::new(update.element, update.scenario));
    }
}

/// Count the number of scenarios an element is included in with the Visibility modifier
pub fn count_scenarios_with_visibility(
    scenarios: &Query<(Entity, &ScenarioModifiers, &Affiliation), With<ScenarioMarker>>,
    element: Entity,
    get_modifier: &GetModifier<Modifier<Visibility>>,
) -> i32 {
    scenarios.iter().fold(0, |x, (e, _, _)| {
        match get_modifier
            .get(e, element)
            .map(|m| **m)
            .unwrap_or(Visibility::Hidden)
        {
            Visibility::Hidden => x,
            _ => x + 1,
        }
    })
}

/// Count the number of scenarios an element is included in with the Inclusion modifier
pub fn count_scenarios_with_inclusion(
    scenarios: &Query<(Entity, &ScenarioModifiers, &Affiliation), With<ScenarioMarker>>,
    element: Entity,
    get_modifier: &GetModifier<Modifier<Inclusion>>,
) -> i32 {
    scenarios.iter().fold(0, |x, (e, _, _)| {
        match get_modifier
            .get(e, element)
            .map(|m| **m)
            .unwrap_or(Inclusion::Hidden)
        {
            Inclusion::Hidden => x,
            _ => x + 1,
        }
    })
}

/// Create a new scenario and its children entities
pub fn handle_create_scenarios(
    mut commands: Commands,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut new_scenarios: EventReader<CreateScenario>,
    current_workspace: Res<CurrentWorkspace>,
) {
    for new in new_scenarios.read() {
        let mut cmd = commands.spawn((
            ScenarioBundle::new(new.name.clone(), new.parent.clone()),
            ScenarioModifiers::default(),
        ));

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
    mut create_new_scenario: EventWriter<CreateScenario>,
    mut delete: EventWriter<Delete>,
    mut scenarios: Query<(Entity, &Affiliation, Option<&mut Dependents>), With<ScenarioMarker>>,
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
    mut validate_events: EventReader<ValidateWorkspace>,
    get_modifier: GetModifier<Modifier<Visibility>>,
    instances: Query<(Entity, &NameInSite, &Affiliation), (With<ModelMarker>, Without<Group>)>,
    scenarios: Query<(Entity, &ScenarioModifiers, &Affiliation), With<ScenarioMarker>>,
) {
    for root in validate_events.read() {
        for (instance_entity, instance_name, _) in instances.iter() {
            if count_scenarios_with_visibility(&scenarios, instance_entity, &get_modifier) > 0 {
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
