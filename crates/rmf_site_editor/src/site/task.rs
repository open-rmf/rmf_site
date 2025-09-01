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

use crate::site::{Element, StandardProperty, Task, TaskKind, TaskParams};
use bevy::prelude::*;
use std::collections::HashMap;

pub type InsertTaskKindFn = fn(EntityCommands);
pub type RemoveTaskKindFn = fn(EntityCommands);
pub type IsTaskValidFn = fn(Entity, &World) -> bool;

#[derive(Resource)]
pub struct TaskKinds(pub HashMap<String, (InsertTaskKindFn, RemoveTaskKindFn, IsTaskValidFn)>);

impl FromWorld for TaskKinds {
    fn from_world(_world: &mut World) -> Self {
        TaskKinds(HashMap::new())
    }
}

impl Element for Task {}

impl StandardProperty for TaskParams {}

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
