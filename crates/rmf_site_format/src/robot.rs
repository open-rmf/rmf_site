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
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component), require(OnLevel<Entity>))]
pub struct Robot {
    // TODO(@xiyuoh) Fleet name is a string for now, we probably want some kind of
    // fleet registration at some point
    pub fleet: String,
    pub properties: BTreeMap<String, serde_json::Value>,
}

impl Default for Robot {
    fn default() -> Self {
        Self {
            fleet: "<Unnamed>".to_string(),
            properties: BTreeMap::new(),
        }
    }
}
