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
    GoToPlace(SiteParent<T>),
    WaitFor(f32),
}

impl<T: RefTrait> Default for Task<T> {
    fn default() -> Self {
        Self::WaitFor(0.0)
    }
}

impl<T: RefTrait> Task<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Task<U>, T> {
        Ok(match self {
            Task::GoToPlace(affiliation) => Task::GoToPlace(affiliation.convert(id_map)?),
            Task::WaitFor(time) => Task::WaitFor(*time),
        })
    }
}

impl<T: RefTrait> Task<T> {
    pub fn labels() -> Vec<&'static str> {
        vec!["Go To Place", "Wait For"]
    }

    pub fn is_valid(&self) -> bool {
        match self {
            Task::GoToPlace(affiliation) => affiliation.0.is_some(),
            Task::WaitFor(time) => *time >= 0.0,
        }
    }

    pub fn label(&self) -> &'static str {
        match self {
            Task::GoToPlace(_) => Self::labels()[0],
            Task::WaitFor(_) => Self::labels()[1],
        }
    }

    pub fn from_label(label: &str) -> Self {
        let labels = Self::labels();
        match labels.iter().position(|&l| l == label) {
            Some(0) => Task::GoToPlace(SiteParent::<T>(None)),
            Some(1) => Task::WaitFor(0.0),
            _ => Task::WaitFor(0.0),
        }
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct DifferentialDrive {
    pub translational_speed: f32,
    pub rotational_speed: f32,
    pub bidirectional: bool,
    pub rotation_center_offset: [f32; 2],
}

impl Default for DifferentialDrive {
    fn default() -> Self {
        Self {
            translational_speed: 0.5,
            rotational_speed: 1.0,
            bidirectional: false,
            rotation_center_offset: [0.0, 0.0],
        }
    }
}