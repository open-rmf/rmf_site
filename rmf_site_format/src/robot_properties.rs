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
use serde_json::{Map, Value};
use thiserror::Error;

#[cfg(feature = "bevy")]
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
    fn new(kind: String, config: Value) -> Self;

    fn is_default(&self) -> bool;

    fn kind(&self) -> Option<String>;

    fn label() -> String;
}

#[cfg(feature = "bevy")]
pub trait RobotPropertyKind:
    'static + Send + Sync + Default + Clone + Component + PartialEq + Serialize + DeserializeOwned
{
    fn label() -> String;
}

#[cfg(feature = "bevy")]
pub trait RecallPropertyKind: Recall + Default + Component<Mutability = Mutable> {
    type Kind: RobotPropertyKind;
    fn assume(&self) -> Self::Kind;
}

#[derive(Debug, Error)]
pub enum RobotPropertyError {
    #[error("Robot property [{0}] is not found on this robot")]
    PropertyNotFound(String),
    #[error("Robot property kind [{0}] is not found in this robot")]
    PropertyKindNotFound(String),
    #[error("Failed serializing robot property kind: {0}")]
    SerializePropertyError(String),
    #[error("Failed serializing robot property: {0}")]
    SerializePropertyKindError(String),
    #[error("Failed deserializing robot property: {0}")]
    DeserializePropertyError(String),
    #[error("Failed deserializing robot property kind: {0}")]
    DeserializePropertyKindError(String),
}

#[cfg(feature = "bevy")]
pub fn serialize_robot_property<Property: RobotProperty>(
    property: Property,
) -> Result<Value, RobotPropertyError> {
    serde_json::to_value(property)
        .map_err(|_| RobotPropertyError::SerializePropertyError(Property::label()))
}

#[cfg(feature = "bevy")]
pub fn serialize_robot_property_from_kind<Property: RobotProperty, Kind: RobotPropertyKind>(
    property_kind: Kind,
) -> Result<Value, RobotPropertyError> {
    let label = Kind::label();
    serde_json::to_value(property_kind)
        .map_err(|_| RobotPropertyError::SerializePropertyKindError(label.clone()))
        .map(|val| Property::new(label, val))
        .and_then(|new_property| serialize_robot_property::<Property>(new_property))
}

#[cfg(feature = "bevy")]
pub fn serialize_robot_property_kind<Property: RobotProperty, Kind: RobotPropertyKind>(
    property_kind: Kind,
) -> Result<Value, RobotPropertyError> {
    let label = Kind::label();
    serde_json::to_value(property_kind)
        .map_err(|_| RobotPropertyError::SerializePropertyKindError(label))
}

#[cfg(feature = "bevy")]
pub fn deserialize_robot_property<Property: RobotProperty>(
    value: Value,
) -> Result<Property, RobotPropertyError> {
    serde_json::from_value::<Property>(value)
        .map_err(|_| RobotPropertyError::DeserializePropertyError(Property::label()))
}

#[cfg(feature = "bevy")]
pub fn deserialize_robot_property_kind<Kind: RobotPropertyKind>(
    value: Value,
) -> Result<Kind, RobotPropertyError> {
    serde_json::from_value::<Kind>(value)
        .map_err(|_| RobotPropertyError::DeserializePropertyKindError(Kind::label()))
}

#[cfg(feature = "bevy")]
/// Returns the specified RobotProperty if it is present in this robot
pub fn retrieve_robot_property<Property: RobotProperty>(
    robot: Robot,
) -> Result<(Property, Value), RobotPropertyError> {
    let property_label = Property::label();

    match robot.properties.get(&property_label) {
        Some(property_value) => {
            if property_value.as_object().is_none_or(|m| m.is_empty()) {
                // Robot property does not have/require any nested value, return default
                // TODO(@xiyuoh) check again why do we want to force this to be Value::Object
                Ok((Property::default(), Value::Object(Map::new())))
            } else {
                deserialize_robot_property::<Property>(property_value.clone())
                    .map(|property| (property.clone(), property_value.clone()))
            }
        }
        None => Err(RobotPropertyError::PropertyNotFound(property_label)),
    }
}

/// Returns the specified RobotPropertyKind if it is present in this serialized RobotProperty
#[cfg(feature = "bevy")]
pub fn retrieve_robot_property_kind<
    Kind: RobotPropertyKind,
    Property: RobotProperty,
    RecallKind: RecallPropertyKind<Kind = Kind>,
>(
    value: Value,
    recall_kind: Option<&RecallKind>,
) -> Result<Kind, RobotPropertyError> {
    if let Some(property) = value.as_object() {
        if property
            .get("kind")
            .and_then(|kind| kind.as_str())
            .is_some_and(|label| label == Kind::label().as_str())
        {
            let kind = match property
                .get("config")
                .and_then(|config| deserialize_robot_property_kind::<Kind>(config.clone()).ok())
            {
                Some(property_kind) => {
                    if let Some(recall) = recall_kind.filter(|_| property_kind == Kind::default()) {
                        recall.assume()
                    } else {
                        property_kind
                    }
                }
                None => Kind::default(),
            };
            return Ok(kind);
        }
    }
    Err(RobotPropertyError::PropertyKindNotFound(Kind::label()))
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Mobility {
    pub kind: String,
    pub config: Value,
}

impl Default for Mobility {
    fn default() -> Self {
        Self {
            kind: String::new(),
            config: Value::Object(Map::new()),
        }
    }
}

#[cfg(feature = "bevy")]
impl RobotProperty for Mobility {
    fn new(kind: String, config: Value) -> Self {
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
    pub config: Option<Value>,
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

#[cfg(feature = "bevy")]
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

#[cfg(feature = "bevy")]
impl RecallPropertyKind for RecallDifferentialDrive {
    type Kind = DifferentialDrive;

    fn assume(&self) -> DifferentialDrive {
        DifferentialDrive {
            bidirectional: self.bidirectional.clone().unwrap_or(false),
            rotation_center_offset: self.rotation_center_offset.clone().unwrap_or([0.0, 0.0]),
            translational_speed: self.translational_speed.clone().unwrap_or(0.5),
            translational_acceleration: self.translational_acceleration.clone().unwrap_or(0.25),
            rotational_speed: self.rotational_speed.clone().unwrap_or(1.0),
            rotational_acceleration: self.rotational_acceleration.clone().unwrap_or(1.5),
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
    pub config: Value,
}

impl Default for Collision {
    fn default() -> Self {
        Self {
            kind: String::new(),
            config: Value::Object(Map::new()),
        }
    }
}

#[cfg(feature = "bevy")]
impl RobotProperty for Collision {
    fn new(kind: String, config: Value) -> Self {
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
    pub config: Option<Value>,
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

#[cfg(feature = "bevy")]
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

#[cfg(feature = "bevy")]
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
