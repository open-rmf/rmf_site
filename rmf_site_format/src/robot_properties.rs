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

use crate::*;
#[cfg(feature = "bevy")]
use bevy::{
    ecs::component::Mutable,
    prelude::{Component, *},
};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::Map;

pub trait RobotProperty:
    'static
    + Send
    + Sync
    + Default
    + Clone
    + Component<Mutability = Mutable>
    + PartialEq
    + Serialize
    + DeserializeOwned
{
    fn new(kind: String, config: serde_json::Value) -> Self;

    fn is_default(&self) -> bool;

    fn kind(&self) -> Option<String>;

    fn label() -> String;
}

pub trait RobotPropertyKind:
    'static + Send + Sync + Default + Clone + Component + PartialEq + Serialize + DeserializeOwned
{
    fn label() -> String;
}

pub trait RecallPropertyKind: Recall + Default + Component<Mutability = Mutable> {
    type Kind: RobotPropertyKind;
    fn assume(&self) -> Self::Kind;
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
    fn new(kind: String, config: serde_json::Value) -> Self {
        Self { kind, config }
    }

    fn is_default(&self) -> bool {
        if *self == Self::default() {
            return true;
        }
        false
    }

    fn kind(&self) -> Option<String> {
        Some(self.kind.clone())
    }

    fn label() -> String {
        "Mobility".to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallMobility {
    pub kind: Option<String>,
    pub config: Option<serde_json::Value>,
}

impl Recall for RecallMobility {
    type Source = Mobility;

    fn remember(&mut self, source: &Mobility) {
        self.kind = Some(source.kind.clone());
        self.config = Some(source.config.clone());
    }
}

// Supported kinds of Mobility
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct DifferentialDrive {
    pub bidirectional: bool,
    pub rotation_center_offset: [f32; 2],
    pub translational_speed: f32,
    pub translational_acceleration: f32,
    pub rotational_speed: f32,
    pub rotational_acceleration: f32,
}

impl Default for DifferentialDrive {
    fn default() -> Self {
        Self {
            bidirectional: false,
            rotation_center_offset: [0.0, 0.0],
            translational_speed: 0.5,
            translational_acceleration: 0.25,
            rotational_speed: 1.0,
            rotational_acceleration: 1.5,
        }
    }
}

impl RobotPropertyKind for DifferentialDrive {
    fn label() -> String {
        "Differential Drive".to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallDifferentialDrive {
    pub bidirectional: Option<bool>,
    pub rotation_center_offset: Option<[f32; 2]>,
    pub translational_speed: Option<f32>,
    pub translational_acceleration: Option<f32>,
    pub rotational_speed: Option<f32>,
    pub rotational_acceleration: Option<f32>,
}

impl RecallPropertyKind for RecallDifferentialDrive {
    type Kind = DifferentialDrive;

    fn assume(&self) -> DifferentialDrive {
        DifferentialDrive {
            bidirectional: self.bidirectional.clone().unwrap_or_default(),
            rotation_center_offset: self.rotation_center_offset.clone().unwrap_or_default(),
            translational_speed: self.translational_speed.clone().unwrap_or_default(),
            translational_acceleration: self.translational_acceleration.clone().unwrap_or_default(),
            rotational_speed: self.rotational_speed.clone().unwrap_or_default(),
            rotational_acceleration: self.rotational_acceleration.clone().unwrap_or_default(),
        }
    }
}

impl Recall for RecallDifferentialDrive {
    type Source = DifferentialDrive;

    fn remember(&mut self, source: &DifferentialDrive) {
        self.bidirectional = Some(source.bidirectional);
        self.rotation_center_offset = Some(source.rotation_center_offset);
        self.translational_speed = Some(source.translational_speed);
        self.translational_acceleration = Some(source.translational_acceleration);
        self.rotational_speed = Some(source.rotational_speed);
        self.rotational_acceleration = Some(source.rotational_acceleration);
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
    fn new(kind: String, config: serde_json::Value) -> Self {
        Self { kind, config }
    }

    fn is_default(&self) -> bool {
        if *self == Self::default() {
            return true;
        }
        false
    }

    fn kind(&self) -> Option<String> {
        Some(self.kind.clone())
    }

    fn label() -> String {
        "Collision".to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallCollision {
    pub kind: Option<String>,
    pub config: Option<serde_json::Value>,
}

impl Recall for RecallCollision {
    type Source = Collision;

    fn remember(&mut self, source: &Collision) {
        self.kind = Some(source.kind.clone());
        self.config = Some(source.config.clone());
    }
}

// Supported kinds of Collision
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct CircleCollision {
    pub radius: f32,
    pub offset: [f32; 2],
}

impl Default for CircleCollision {
    fn default() -> Self {
        Self {
            radius: 0.0,
            offset: [0.0, 0.0],
        }
    }
}

impl RobotPropertyKind for CircleCollision {
    fn label() -> String {
        "Circle Collision".to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallCircleCollision {
    pub radius: Option<f32>,
    pub offset: Option<[f32; 2]>,
}

impl RecallPropertyKind for RecallCircleCollision {
    type Kind = CircleCollision;

    fn assume(&self) -> CircleCollision {
        CircleCollision {
            radius: self.radius.clone().unwrap_or_default(),
            offset: self.offset.clone().unwrap_or_default(),
        }
    }
}

impl Recall for RecallCircleCollision {
    type Source = CircleCollision;

    fn remember(&mut self, source: &CircleCollision) {
        self.radius = Some(source.radius);
        self.offset = Some(source.offset);
    }
}
