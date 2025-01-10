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

#[cfg(feature = "bevy")]
use bevy::prelude::{Component, Reflect, ReflectComponent};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct RobotMarker;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Mobility {
    pub kind: String,
    pub config: serde_json::Value,
    pub bidirectional: bool,
    pub collision_radius: f32,
    pub rotation_center_offset: [f32; 2],
}

impl Default for Mobility {
    fn default() -> Self {
        Self {
            kind: String::new(),
            config: serde_json::Value::Null,
            bidirectional: false,
            collision_radius: 0.5,
            rotation_center_offset: [0.0, 0.0],
        }
    }
}

impl Mobility {
    pub fn is_default(&self) -> bool {
        if *self == Mobility::default() {
            return true;
        }
        false
    }

    pub fn is_empty(&self) -> bool {
        if self.kind.is_empty() {
            return true;
        } else if self.config.is_null() {
            return true;
        }
        false
    }
}

// Supported kinds of Mobility
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct DifferentialDrive {
    pub translational_speed: f32,
    pub rotational_speed: f32,
}

impl Default for DifferentialDrive {
    fn default() -> Self {
        Self {
            translational_speed: 0.5,
            rotational_speed: 1.0,
        }
    }
}

impl DifferentialDrive {
    pub fn label() -> String {
        "Differential Drive".to_string()
    }
}
