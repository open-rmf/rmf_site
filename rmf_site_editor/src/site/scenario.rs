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
        AddModifier, Affiliation, CurrentScenario, Delete, Dependents, GetModifier, Group,
        InheritedInstance, InstanceMarker, InstanceModifier, IssueKey, LastSetValue, ModelMarker,
        Modifier, NameInSite, Pending, PendingModel, Pose, Property, RecallInstance,
        RemoveModifier, ScenarioBundle, ScenarioMarker, ScenarioModifiers, UpdateProperty,
    },
    widgets::view_model_instances::count_scenarios,
    CurrentWorkspace, Issue, ValidateWorkspace,
};
use bevy::ecs::{
    hierarchy::ChildOf,
    system::{SystemParam, SystemState},
};
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

#[derive(Clone, Debug, Copy)]
pub enum UpdateInstance {
    Include,
    Hide,
    Modify(Pose),
    ResetPose,
    ResetVisibility,
}

#[derive(Clone, Debug, Event, Copy)]
pub struct UpdateInstanceEvent {
    pub scenario: Entity,
    pub instance: Entity,
    pub update: UpdateInstance,
}

impl Property for Pose {
    fn get_fallback(for_element: Entity, in_scenario: Entity, world: &mut World) -> Pose {
        let mut state: SystemState<(
            Query<&ScenarioModifiers<Entity>, With<ScenarioMarker>>,
            Query<&RecallInstance>,
            Query<&LastSetValue<Pose>>,
        )> = SystemState::new(world);
        let (scenarios, recall_instance, last_set_pose) = state.get(world);

        // Recall instance pose for this scenario if any
        if let Some(recall_pose) = scenarios
            .get(in_scenario)
            .ok()
            .and_then(|scenario_modifiers| scenario_modifiers.get(&for_element))
            .and_then(|modifier_entity| recall_instance.get(*modifier_entity).ok())
            .and_then(|recall_modifier| recall_modifier.pose)
        {
            return recall_pose;
        }

        // Otherwise return the last set pose for this model instance
        last_set_pose
            .get(for_element)
            .map(|value| value.0)
            .unwrap_or(Pose::default())
    }
}

impl Modifier<Pose> for InstanceModifier {
    fn get(&self) -> Option<Pose> {
        self.pose()
    }

    fn insert(for_element: Entity, in_scenario: Entity, value: Pose, world: &mut World) {
        let mut state: SystemState<(
            Query<(&mut InstanceModifier, &Affiliation<Entity>)>,
            Query<
                (Entity, &ScenarioModifiers<Entity>, Ref<Affiliation<Entity>>),
                With<ScenarioMarker>,
            >,
            EventWriter<AddModifier>,
        )> = SystemState::new(world);
        let (mut instance_modifiers, scenarios, _) = state.get_mut(world);

        // Insert instance modifier entities when new model instances are spawned and placed
        let Ok((_, scenario_modifiers, _)) = scenarios.get(in_scenario) else {
            return;
        };
        let mut new_modifiers = Vec::<(InstanceModifier, Entity)>::new();

        if let Some((mut instance_modifier, _)) = scenario_modifiers
            .get(&for_element)
            .and_then(|e| instance_modifiers.get_mut(*e).ok())
        {
            // If an instance modifier entity already exists for this scenario, update it
            let instance_modifier = instance_modifier.as_mut();
            match instance_modifier {
                InstanceModifier::Added(_) => {
                    *instance_modifier = InstanceModifier::added(value.clone())
                }
                InstanceModifier::Inherited(inherited) => {
                    inherited.modified_pose = Some(value.clone())
                }
                InstanceModifier::Hidden => {}
            }
        } else {
            // If instance modifier entity does not exist in this scenario, spawn one
            new_modifiers.push((InstanceModifier::added(value.clone()), in_scenario));
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
        // Insert instance modifier into all root scenarios outside of the current tree as hidden
        for (scenario_entity, _, parent_scenario) in scenarios.iter() {
            if parent_scenario.0.is_some() || scenario_entity == current_root_entity {
                continue;
            }
            new_modifiers.push((InstanceModifier::Hidden, scenario_entity));
        }

        // Spawn all new modifier entities
        let new_modifier_entities = new_modifiers
            .iter()
            .map(|(modifier, scenario)| (world.spawn(modifier.clone()).id(), *scenario))
            .collect::<Vec<(Entity, Entity)>>();
        let (_, _, mut add_modifier) = state.get_mut(world);
        for (modifier_entity, scenario_entity) in new_modifier_entities.iter() {
            add_modifier.write(AddModifier::new(
                for_element,
                *modifier_entity,
                *scenario_entity,
            ));
        }
    }

    fn insert_on_new_scenario(in_scenario: Entity, world: &mut World) {
        let mut state: SystemState<(
            Query<&Children>,
            Query<(&InstanceModifier, &Affiliation<Entity>)>,
            Query<Entity, (With<InstanceMarker>, Without<Pending>)>,
            EventWriter<AddModifier>,
            EventWriter<ChangeCurrentScenario>,
        )> = SystemState::new(world);
        let (children, instance_modifiers, model_instances, _, _) = state.get_mut(world);

        // Insert instance modifier entities when new root scenarios are created
        let mut have_instance = HashSet::new();
        if let Ok(scenario_children) = children.get(in_scenario) {
            for child in scenario_children {
                if let Ok((_, a)) = instance_modifiers.get(*child) {
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
            // Mark all instance modifiers as Hidden
            new_modifiers.push((
                *target,
                world.commands().spawn(InstanceModifier::Hidden).id(),
            ));
        }

        let (_, _, _, mut add_modifier, mut change_current_scenario) = state.get_mut(world);
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

impl Property for Visibility {
    fn get_fallback(_for_element: Entity, _in_scenario: Entity, _world: &mut World) -> Visibility {
        // We want the instance to be hidden by default, and only visible
        // when intentionally toggled
        Visibility::Hidden
    }
}

impl Modifier<Visibility> for InstanceModifier {
    fn get(&self) -> Option<Visibility> {
        self.visibility().map(|v| {
            if v {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        })
    }

    fn retrieve_inherited(
        &self,
        for_element: Entity,
        in_scenario: Entity,
        get_modifier: &GetModifier<InstanceModifier>,
    ) -> Option<Visibility> {
        let mut parent_visibility: Option<bool> = None;
        let mut entity = in_scenario;
        while parent_visibility.is_none() {
            let Some(parent_entity) = get_modifier
                .scenarios
                .get(entity)
                .ok()
                .and_then(|(_, p)| p.0)
            else {
                break;
            };

            if let Some(instance_modifier) = get_modifier.get(parent_entity, for_element) {
                parent_visibility = instance_modifier.visibility();
            }
            entity = parent_entity;
        }
        parent_visibility.map(|v| {
            if v {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            }
        })
    }
}

/// Handles updates when the current scenario has changed, and trigger property updates for scenario elements
pub fn update_current_scenario(
    mut commands: Commands,
    mut change_current_scenario: EventReader<ChangeCurrentScenario>,
    mut current_scenario: ResMut<CurrentScenario>,
    instances: Query<Entity, (With<InstanceMarker>, Without<PendingModel>)>,
) {
    if let Some(ChangeCurrentScenario(scenario_entity)) = change_current_scenario.read().last() {
        *current_scenario = CurrentScenario(Some(*scenario_entity));
        for instance_entity in instances.iter() {
            commands.trigger(UpdateProperty::new(instance_entity, *scenario_entity));
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
        if new_pose.is_changed() && !new_pose.is_added() {
            update_instance.write(UpdateInstanceEvent {
                scenario: current_scenario_entity,
                instance: entity,
                update: UpdateInstance::Modify(new_pose.clone()),
            });
        }
    }
}

#[derive(SystemParam)]
pub struct InstanceParams<'w, 's> {
    add_modifier: EventWriter<'w, AddModifier>,
    remove_modifier: EventWriter<'w, RemoveModifier>,
    instance_modifiers:
        Query<'w, 's, (&'static mut InstanceModifier, &'static Affiliation<Entity>)>,
    recall_instance: Query<'w, 's, &'static RecallInstance>,
    scenarios: Query<
        'w,
        's,
        (
            &'static mut ScenarioModifiers<Entity>,
            &'static Affiliation<Entity>,
        ),
        With<ScenarioMarker>,
    >,
}

/// Handles updates to model instance modifiers for all scenarios
pub fn handle_instance_updates(
    world: &mut World,
    state: &mut SystemState<(EventReader<UpdateInstanceEvent>, InstanceParams)>,
) {
    let (mut update_events, _) = state.get_mut(world);
    if update_events.is_empty() {
        return;
    }

    let mut update_instance = Vec::<(UpdateInstanceEvent, Pose)>::new();
    for update in update_events.read() {
        update_instance.push((*update, Pose::default()));
    }
    for (update, pose) in update_instance.iter_mut() {
        *pose = Pose::get_fallback(update.instance, update.scenario, world);
    }

    for (update, fallback_pose) in update_instance.iter() {
        let (_, mut params) = state.get_mut(world);
        let Ok((scenario_modifiers, scenario_parent)) = params.scenarios.get(update.scenario)
        else {
            continue;
        };

        if let Some((mut instance_modifier, modifier_entity)) =
            scenario_modifiers.get(&update.instance).and_then(|e| {
                params
                    .instance_modifiers
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
                            if let Some(recall_modifier) = params
                                .recall_instance
                                .get(*modifier_entity)
                                .ok()
                                .and_then(|r| r.modifier.clone())
                            {
                                // RecallInstance exists, check for previous
                                match recall_modifier {
                                    InstanceModifier::Added(_) => {
                                        *instance_modifier =
                                            InstanceModifier::added(*fallback_pose);
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
                                *instance_modifier = match scenario_parent.0 {
                                    Some(_) => InstanceModifier::inherited_with_inclusion(),
                                    None => InstanceModifier::added(*fallback_pose),
                                };
                            }
                        }
                    }
                }
                UpdateInstance::Hide => {
                    *instance_modifier = InstanceModifier::Hidden;
                }
                UpdateInstance::Modify(new_pose) => {
                    match instance_modifier {
                        InstanceModifier::Added(_) => {
                            *instance_modifier = InstanceModifier::added(new_pose.clone())
                        }
                        InstanceModifier::Inherited(inherited) => {
                            inherited.modified_pose = Some(new_pose.clone())
                        }
                        InstanceModifier::Hidden => {}
                    };
                    // Do not trigger PropertyPlugin<Pose> if pose for existing modifier
                    // was modified by user
                    world
                        .commands()
                        .entity(update.instance)
                        .insert(LastSetValue::<Pose>::new(new_pose));
                    continue;
                }
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
                        params
                            .remove_modifier
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
            let modifier_entity = world.commands().spawn(instance_modifier).id();
            let (_, mut params) = state.get_mut(world);
            params.add_modifier.write(AddModifier::new(
                update.instance,
                modifier_entity,
                update.scenario,
            ));
            continue;
        }

        world
            .commands()
            .trigger(UpdateProperty::new(update.instance, update.scenario));
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
        let mut cmd = commands.spawn((
            ScenarioBundle::<Entity>::new(new.name.clone(), new.parent.clone()),
            ScenarioModifiers::<Entity>::default(),
        ));

        if let Some(parent) = current_workspace.root {
            cmd.insert(ChildOf(parent));
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
            if count_scenarios(&scenarios, instance_entity, &get_modifier) > 0 {
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
