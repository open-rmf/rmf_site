/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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
        AddModifier, Affiliation, ChangeCurrentScenario, Delete, Inclusion, LastSetValue, Modifier,
        Pending, Property, ScenarioModifiers, Task, TaskKind, TaskParams,
    },
    widgets::tasks::{EditMode, EditModeEvent, EditTask},
    CurrentWorkspace,
};
use bevy::ecs::{hierarchy::ChildOf, system::SystemState};
use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

pub type InsertTaskKindFn = fn(EntityCommands);
pub type RemoveTaskKindFn = fn(EntityCommands);

#[derive(Resource)]
pub struct TaskKinds(pub HashMap<String, (InsertTaskKindFn, RemoveTaskKindFn)>);

impl FromWorld for TaskKinds {
    fn from_world(_world: &mut World) -> Self {
        TaskKinds(HashMap::new())
    }
}

impl Property for TaskParams {
    fn get_fallback(for_element: Entity, _in_scenario: Entity, world: &mut World) -> TaskParams {
        let mut state: SystemState<Query<&LastSetValue<TaskParams>>> = SystemState::new(world);
        let last_set_params = state.get(world);

        last_set_params
            .get(for_element)
            .map(|value| (**value).clone())
            .unwrap_or(TaskParams::default())
    }

    fn insert(for_element: Entity, in_scenario: Entity, value: TaskParams, world: &mut World) {
        // TODO(@xiyuoh) the insert() implementation across Properties are actually quite similar,
        // there is significant overlap between Property impl for TaskParams and Pose. Consider
        // moving this logic into StandardProperty instead
        let mut state: SystemState<(
            Query<(&mut Modifier<TaskParams>, &Affiliation<Entity>)>,
            Query<(Entity, &ScenarioModifiers<Entity>, Ref<Affiliation<Entity>>)>,
        )> = SystemState::new(world);
        let (mut task_modifiers, scenarios) = state.get_mut(world);

        // Insert task modifier entities when new tasks are created
        let Ok((_, scenario_modifiers, _)) = scenarios.get(in_scenario) else {
            return;
        };
        let mut new_task_modifiers = Vec::<(Modifier<TaskParams>, Entity)>::new();
        let mut new_inclusion_modifiers = Vec::<(Modifier<Inclusion>, Entity)>::new();

        if let Some((mut task_modifier, _)) = scenario_modifiers
            .get(&for_element)
            .and_then(|e| task_modifiers.get_mut(*e).ok())
        {
            // If a task modifier entity already exists for this scenario, update it
            **task_modifier = value.clone()
        } else {
            // If root modifier entity does not exist in this scenario, spawn one
            new_task_modifiers.push((Modifier::<TaskParams>::new(value.clone()), in_scenario));
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
        // Insert task modifier into all root scenarios outside of the current tree as hidden
        for (scenario_entity, _, parent_scenario) in scenarios.iter() {
            if parent_scenario.0.is_some() || scenario_entity == current_root_entity {
                continue;
            }
            new_inclusion_modifiers.push((
                Modifier::<Inclusion>::new(Inclusion::Hidden),
                scenario_entity,
            ));
        }

        // Spawn all new modifier entities
        let new_current_scenario_modifiers = new_task_modifiers
            .iter()
            .map(|(modifier, scenario)| {
                (
                    world
                        .spawn(modifier.clone())
                        // Mark all newly spawned instances as included
                        .insert(Modifier::<Inclusion>::new(Inclusion::Included))
                        .id(),
                    *scenario,
                )
            })
            .collect::<Vec<(Entity, Entity)>>();
        let mut new_modifier_entities = new_inclusion_modifiers
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

    fn insert_on_new_scenario(in_scenario: Entity, world: &mut World) {
        let mut state: SystemState<(
            Query<&Children>,
            Query<(&Modifier<Inclusion>, &Affiliation<Entity>)>,
            Query<Entity, (With<Task>, Without<Pending>)>,
        )> = SystemState::new(world);
        let (children, task_modifiers, task_entity) = state.get_mut(world);

        // Insert task modifier entities when new root scenarios are created
        let mut have_task = HashSet::new();
        if let Ok(scenario_children) = children.get(in_scenario) {
            for child in scenario_children {
                if let Ok((_, a)) = task_modifiers.get(*child) {
                    if let Some(a) = a.0 {
                        have_task.insert(a);
                    }
                }
            }
        }

        let mut target_tasks = HashSet::new();
        for task_entity in task_entity.iter() {
            if !have_task.contains(&task_entity) {
                target_tasks.insert(task_entity);
            }
        }

        let mut new_modifiers = Vec::<(Entity, Entity)>::new();
        for target in target_tasks.iter() {
            // Mark all task modifiers as Hidden
            new_modifiers.push((
                *target,
                world
                    .commands()
                    .spawn(Modifier::<Inclusion>::new(Inclusion::Hidden))
                    .id(),
            ));
        }

        let mut events_state: SystemState<(
            EventWriter<AddModifier>,
            EventWriter<ChangeCurrentScenario>,
        )> = SystemState::new(world);
        let (mut add_modifier, mut change_current_scenario) = events_state.get_mut(world);
        for (task_entity, modifier_entity) in new_modifiers.iter() {
            add_modifier.write(AddModifier::new(
                *task_entity,
                *modifier_entity,
                in_scenario,
            ));
        }
        change_current_scenario.write(ChangeCurrentScenario(in_scenario));
    }
}

/// Updates the current EditTask entity based on the triggered edit mode event
pub fn handle_task_edit(
    mut commands: Commands,
    mut delete: EventWriter<Delete>,
    mut edit_mode: EventReader<EditModeEvent>,
    mut edit_task: ResMut<EditTask>,
    pending_tasks: Query<&mut Task, With<Pending>>,
    current_workspace: Res<CurrentWorkspace>,
) {
    // TODO(@xiyuoh) fix bug where the egui panel glitches when the EditTask resource is being accessed
    if let Some(edit) = edit_mode.read().last() {
        match edit.mode {
            EditMode::New(task_entity) => {
                if let Some(site_entity) = current_workspace.root {
                    commands.entity(task_entity).insert(ChildOf(site_entity));
                }
                edit_task.0 = Some(task_entity);
            }
            EditMode::Edit(task_entity) => {
                if let Some(pending_task) = edit_task.0.filter(|e| pending_tasks.get(*e).is_ok()) {
                    delete.write(Delete::new(pending_task));
                }
                edit_task.0 = task_entity;
            }
        }
    }
}

pub fn update_task_kind_component<T: TaskKind>(
    mut commands: Commands,
    tasks: Query<(Entity, Ref<Task>, Option<&T>)>,
) {
    for (entity, task, task_kind) in tasks.iter() {
        if task.is_changed() {
            let task_request = task.request();
            if task_request.category() == T::label() && task_kind.is_none() {
                // This TaskKind is present in the task but the component has not been inserted
                if let Ok(task_kind_component) =
                    serde_json::from_value::<T>(task_request.description())
                {
                    commands.entity(entity).insert(task_kind_component);
                }
            }
        }
    }
}
