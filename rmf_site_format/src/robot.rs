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
use bevy::{
    math::Vec2,
    prelude::{Component, Reflect},
};
use serde::{Deserialize, Serialize};
use serde_json::Map;
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
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

pub trait RobotProperty {
    fn is_default(&self) -> bool;

    fn is_empty(&self) -> bool;

    fn kind(&self) -> String;

    fn kind_mut(&mut self) -> &mut String;

    fn config_mut(&mut self) -> &mut serde_json::Value;

    fn label() -> String;
}

pub trait RobotPropertyKind {
    fn label() -> String;
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Mobility {
    pub kind: String,
    pub config: serde_json::Value,
}

impl Default for Mobility {
    fn default() -> Self {
        Self {
            kind: String::new(),
            config: serde_json::Value::Object(Map::new()),
        }
    }
}

impl RobotProperty for Mobility {
    fn is_default(&self) -> bool {
        if *self == Self::default() {
            return true;
        }
        false
    }

    fn is_empty(&self) -> bool {
        if self.kind.is_empty() {
            return true;
        }
        return self.config.as_object().is_none_or(|m| m.is_empty());
    }

    fn kind(&self) -> String {
        self.kind.clone()
    }

    fn kind_mut(&mut self) -> &mut String {
        &mut self.kind
    }

    fn config_mut(&mut self) -> &mut serde_json::Value {
        &mut self.config
    }

    fn label() -> String {
        "Mobility".to_string()
    }
}

// Supported kinds of Mobility
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct DifferentialDrive {
    pub bidirectional: bool,
    pub collision_radius: f32,
    pub rotation_center_offset: [f32; 2],
    pub translational_speed: f32,
    pub rotational_speed: f32,
}

impl Default for DifferentialDrive {
    fn default() -> Self {
        Self {
            bidirectional: false,
            collision_radius: 0.5,
            rotation_center_offset: [0.0, 0.0],
            translational_speed: 0.5,
            rotational_speed: 1.0,
        }
    }
}

impl RobotPropertyKind for DifferentialDrive {
    fn label() -> String {
        "Differential Drive".to_string()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Collision {
    pub kind: String,
    pub config: serde_json::Value,
}

impl Default for Collision {
    fn default() -> Self {
        Self {
            kind: String::new(),
            config: serde_json::Value::Object(Map::new()),
        }
    }
}

impl RobotProperty for Collision {
    fn is_default(&self) -> bool {
        if *self == Self::default() {
            return true;
        }
        false
    }

    fn is_empty(&self) -> bool {
        if self.kind.is_empty() {
            return true;
        }
        return self.config.as_object().is_none_or(|m| m.is_empty());
    }

    fn kind(&self) -> String {
        self.kind.clone()
    }

    fn kind_mut(&mut self) -> &mut String {
        &mut self.kind
    }

    fn config_mut(&mut self) -> &mut serde_json::Value {
        &mut self.config
    }

    fn label() -> String {
        "Collision".to_string()
    }
}

// Supported kinds of Collision
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
pub struct CircleCollision {
    pub radius: f32,
    pub offset: Vec2,
}

impl Default for CircleCollision {
    fn default() -> Self {
        Self {
            radius: 0.0,
            offset: Vec2::default(),
        }
    }
}

impl RobotPropertyKind for CircleCollision {
    fn label() -> String {
        "Circle Collision".to_string()
    }
}
