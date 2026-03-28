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

use crate::site::{
    Element, Group, ModelMarker, Robot, StandardProperty, Task, TaskKind, TaskParams,
};
use bevy::prelude::*;
use std::collections::HashMap;

pub type InsertTaskKindFn = fn(EntityCommands);
pub type RemoveTaskKindFn = fn(EntityCommands);
pub type IsTaskValidFn = fn(Entity, &mut World) -> bool;

#[derive(Resource)]
pub struct TaskKinds(pub HashMap<String, (InsertTaskKindFn, RemoveTaskKindFn, IsTaskValidFn)>);

impl FromWorld for TaskKinds {
    fn from_world(_world: &mut World) -> Self {
        TaskKinds(HashMap::new())
    }
}

impl Element for Task<Entity> {}

impl StandardProperty for TaskParams {}

pub fn update_task_kind_component<T: TaskKind>(
    mut commands: Commands,
    tasks: Query<(Entity, Ref<Task<Entity>>, Option<&T>)>,
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

// This systems monitors for changes in a Robot's fleet and updates relevant
// RobotTaskRequests accordingly
// TODO(@xiyuoh) This does not update fleet name for DispatchTasks, since they
// are not tagged to any robot. Convert fleet name to its own component so that
// we can track non-direct task fleet name changes too.
pub fn update_direct_task_fleet(
    robots: Query<(Entity, Ref<Robot>), (With<ModelMarker>, Without<Group>)>,
    mut tasks: Query<&mut Task<Entity>>,
) {
    for (entity, robot) in robots.iter() {
        if robot.is_changed() {
            for mut task in tasks.iter_mut() {
                if task.robot().0.is_some_and(|e| e == entity) && task.fleet() != robot.fleet {
                    // Update fleet name if it has changed
                    if let Some(fleet) = task.fleet_mut() {
                        *fleet = robot.fleet.clone();
                    }
                }
            }
        }
    }
}
