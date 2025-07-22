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
    site::{Delete, Element, Pending, StandardProperty, Task, TaskKind, TaskParams},
    widgets::tasks::{EditMode, EditModeEvent, EditTask},
    CurrentWorkspace,
};
use bevy::ecs::hierarchy::ChildOf;
use bevy::prelude::*;
use std::collections::HashMap;

pub type InsertTaskKindFn = fn(EntityCommands);
pub type RemoveTaskKindFn = fn(EntityCommands);

#[derive(Resource)]
pub struct TaskKinds(pub HashMap<String, (InsertTaskKindFn, RemoveTaskKindFn)>);

impl FromWorld for TaskKinds {
    fn from_world(_world: &mut World) -> Self {
        TaskKinds(HashMap::new())
    }
}

impl Element for Task {}

impl StandardProperty for TaskParams {}

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
