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
use std::{collections::HashMap, fmt};
use strum_macros::EnumIter;

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct SimpleTask(pub Option<Pose>);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct GoToPlace<T: RefTrait> {
    pub location: Option<Point<T>>,
}

impl<T: RefTrait> Default for GoToPlace<T> {
    fn default() -> Self {
        Self { location: None }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Reflect)]
pub struct WaitFor {
    pub duration: f32,
}

impl Default for WaitFor {
    fn default() -> Self {
        Self { duration: 0.0 }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, EnumIter, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub enum Task<T: RefTrait> {
    GoToPlace(GoToPlace<T>),
    WaitFor(WaitFor),
    // TODO(@xiyuoh) create structs for these
    // PickUp { payload: Affiliation<T>, location: Point<T>},
    // DropOff { payload: Affiliation<T>, location: Point<T>},
}

impl<T: RefTrait> Default for Task<T> {
    fn default() -> Self {
        Self::WaitFor(WaitFor { duration: 0.0 })
    }
}

impl<T: RefTrait> fmt::Display for Task<T> {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Task::GoToPlace(..) => write!(f, "Go To Place"),
            Task::WaitFor(..) => write!(f, "Wait For"),
        }
    }
}

impl<T: RefTrait> Task<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Task<U>, T> {
        Ok(match self {
            Task::GoToPlace(go_to_place) => Task::GoToPlace(GoToPlace {
                location: Some(go_to_place.location.unwrap().convert(id_map)?),
            }),
            Task::WaitFor(wait_for) => Task::WaitFor(WaitFor {
                duration: wait_for.duration,
            }),
        })
    }
}

impl<T: RefTrait> Task<T> {
    pub fn label(&self) -> String {
        format!("{}", self)
    }

    pub fn location(&self) -> Option<&Point<T>> {
        match self {
            Task::GoToPlace(go_to_place) => go_to_place.location.as_ref(),
            _ => None,
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
