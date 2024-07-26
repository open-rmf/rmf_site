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

use crate::*;
#[cfg(feature = "bevy")]
use bevy::prelude::{Component, Reflect, ReflectComponent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct SimpleTask(pub Option<Pose>);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub enum Task<T: RefTrait> {
    GoToPlace(Pose, SiteParent<T>),
    WaitFor(f32),
    PickUp(Affiliation<T>),
    DropOff((Affiliation<T>, Pose)),
}

impl<T: RefTrait> Default for Task<T> {
    fn default() -> Self {
        Self::WaitFor(0.0)
    }
}

impl<T: RefTrait> Task<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Task<U>, T> {
        Ok(match self {
            Task::GoToPlace(pose, parent) => Task::GoToPlace(*pose, parent.convert(id_map)?),
            Task::WaitFor(time) => Task::WaitFor(*time),
            Task::PickUp(affiliation) => Task::PickUp(affiliation.convert(id_map)?),
            Task::DropOff((affiliation, pose)) => {
                Task::DropOff((affiliation.convert(id_map)?, *pose))
            }
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct Tasks<T: RefTrait>(pub Vec<Task<T>>);

impl<T: RefTrait> Default for Tasks<T> {
    fn default() -> Self {
        Self(Vec::new())
    }
}

impl<T: RefTrait> Tasks<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Tasks<U>, T> {
        self.0.iter().try_fold(Tasks::default(), |mut tasks, task| {
            tasks.0.push(task.convert(id_map)?);
            Ok(tasks)
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct MobileRobotMarker;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct DifferentialDrive {
    pub translational_speed: f32,
    pub rotational_speed: f32,
    pub bidirectional: bool,
    pub rotation_center_offset: [f32; 2],
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct HolonomicDrive {
    pub translational_speed: f32,
    pub rotational_speed: f32,
    pub rotation_center_offset: [f32; 3],
}
