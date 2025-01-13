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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct RobotMarker;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Task {
    pub kind: String,
    pub config: serde_json::Value,
}

impl Default for Task {
    fn default() -> Self {
        Self {
            kind: "Select Kind".to_string(),
            config: serde_json::Value::Null,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Tasks(pub Vec<Task>);

impl Default for Tasks {
    fn default() -> Self {
        Self(Vec::new())
    }
}

// Supported Task kinds
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct GoToPlace {
    pub location: NameInSite,
}

impl Default for GoToPlace {
    fn default() -> Self {
        Self {
            location: NameInSite::default(),
        }
    }
}

impl GoToPlace {
    pub fn is_default(&self) -> bool {
        if *self == GoToPlace::default() {
            return true;
        }
        false
    }

    pub fn label() -> String {
        "Go To Place".to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct WaitFor {
    pub duration: f32,
}

impl Default for WaitFor {
    fn default() -> Self {
        Self { duration: 0.0 }
    }
}

impl WaitFor {
    pub fn label() -> String {
        "Wait For".to_string()
    }
}
