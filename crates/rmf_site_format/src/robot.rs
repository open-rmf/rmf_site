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
use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct RobotLevel<T: RefTrait>(pub Option<T>);

impl<T: RefTrait> Default for RobotLevel<T> {
    fn default() -> Self {
        RobotLevel(None)
    }
}

impl<T: RefTrait> RobotLevel<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<RobotLevel<U>, T> {
        if let Some(x) = self.0 {
            Ok(RobotLevel(Some(id_map.get(&x).ok_or(x)?.clone())))
        } else {
            Ok(RobotLevel(None))
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
#[cfg_attr(feature = "bevy", require(RobotLevel<Entity>))]
pub struct Robot {
    pub properties: HashMap<String, serde_json::Value>,
}

impl Default for Robot {
    fn default() -> Self {
        Self {
            properties: HashMap::new(),
        }
    }
}
