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
use sdformat_rs::{ElementData, ElementMap};
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
pub fn serialize_robot_property_kind<Kind: RobotPropertyKind>(
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
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct Mobility {
    pub kind: String,
    #[reflect(ignore)]
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
    pub rotational_speed: f32,
}

impl Default for DifferentialDrive {
    fn default() -> Self {
        Self {
            bidirectional: false,
            rotation_center_offset: [0.0, 0.0],
            translational_speed: 0.5,
            rotational_speed: 1.0,
        }
    }
}

#[cfg(feature = "bevy")]
impl RobotPropertyKind for DifferentialDrive {
    fn label() -> String {
        "Differential Drive".to_string()
    }
}

#[cfg(feature = "bevy")]
impl From<&ElementMap> for DifferentialDrive {
    fn from(elements: &ElementMap) -> Self {
        let mut diff_drive = DifferentialDrive::default();
        if let Some(reversible) = elements.get("reversible") {
            if let ElementData::String(reversible_str) = &reversible.data {
                diff_drive.bidirectional = if reversible_str == "true" {
                    true
                } else if reversible_str == "false" {
                    false
                } else {
                    warn!(
                        "Found invalid slotcar reversibility data {:?}, 
                                        setting DifferentialDrive reversibility to false.",
                        reversible_str
                    );
                    false
                };
            }
        }
        if let Some(translational_speed) = elements
            .get("nominal_drive_speed")
            .and_then(|speed| f64::try_from(speed.data.clone()).ok())
        {
            diff_drive.translational_speed = translational_speed as f32;
        }
        if let Some(rotational_speed) = elements
            .get("nominal_turn_speed")
            .and_then(|speed| f64::try_from(speed.data.clone()).ok())
        {
            diff_drive.rotational_speed = rotational_speed as f32;
        }

        diff_drive
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallDifferentialDrive {
    pub bidirectional: Option<bool>,
    pub rotation_center_offset: Option<[f32; 2]>,
    pub translational_speed: Option<f32>,
    pub rotational_speed: Option<f32>,
}

#[cfg(feature = "bevy")]
impl RecallPropertyKind for RecallDifferentialDrive {
    type Kind = DifferentialDrive;

    fn assume(&self) -> DifferentialDrive {
        DifferentialDrive {
            bidirectional: self.bidirectional.clone().unwrap_or(false),
            rotation_center_offset: self.rotation_center_offset.clone().unwrap_or([0.0, 0.0]),
            translational_speed: self.translational_speed.clone().unwrap_or(0.5),
            rotational_speed: self.rotational_speed.clone().unwrap_or(1.0),
        }
    }
}

impl Recall for RecallDifferentialDrive {
    type Source = DifferentialDrive;

    fn remember(&mut self, source: &DifferentialDrive) {
        self.bidirectional = Some(source.bidirectional);
        self.rotation_center_offset = Some(source.rotation_center_offset);
        self.translational_speed = Some(source.translational_speed);
        self.rotational_speed = Some(source.rotational_speed);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct Collision {
    pub kind: String,
    #[reflect(ignore)]
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

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct PowerSource {
    pub kind: String,
    #[reflect(ignore)]
    pub config: Value,
}

impl Default for PowerSource {
    fn default() -> Self {
        Self {
            kind: String::new(),
            config: Value::Object(Map::new()),
        }
    }
}

#[cfg(feature = "bevy")]
impl RobotProperty for PowerSource {
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
        "Power Source".to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallPowerSource {
    pub kind: Option<String>,
    pub config: Option<Value>,
}

impl Recall for RecallPowerSource {
    type Source = PowerSource;

    fn remember(&mut self, source: &PowerSource) {
        self.kind = Some(source.kind.clone());
        self.config = Some(source.config.clone());
    }
}

// Supported kinds of Power Source
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct Battery {
    pub voltage: f32,
    pub capacity: f32,
    pub charging_current: f32,
}

impl Default for Battery {
    fn default() -> Self {
        Self {
            voltage: 12.0,
            capacity: 24.0,
            charging_current: 5.0,
        }
    }
}

#[cfg(feature = "bevy")]
impl RobotPropertyKind for Battery {
    fn label() -> String {
        "Battery".to_string()
    }
}

#[cfg(feature = "bevy")]
impl From<&ElementMap> for Battery {
    fn from(elements: &ElementMap) -> Self {
        let mut battery = Battery::default();
        if let Some(voltage) = elements
            .get("nominal_voltage")
            .and_then(|voltage| f64::try_from(voltage.data.clone()).ok())
        {
            battery.voltage = voltage as f32;
        }
        if let Some(capacity) = elements
            .get("nominal_capacity")
            .and_then(|capacity| f64::try_from(capacity.data.clone()).ok())
        {
            battery.capacity = capacity as f32;
        }
        if let Some(current) = elements
            .get("charging_current")
            .and_then(|current| f64::try_from(current.data.clone()).ok())
        {
            battery.charging_current = current as f32;
        }

        battery
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallBattery {
    pub voltage: Option<f32>,
    pub capacity: Option<f32>,
    pub charging_current: Option<f32>,
}

#[cfg(feature = "bevy")]
impl RecallPropertyKind for RecallBattery {
    type Kind = Battery;

    fn assume(&self) -> Battery {
        Battery {
            voltage: self.voltage.clone().unwrap_or(12.0),
            capacity: self.capacity.clone().unwrap_or(24.0),
            charging_current: self.charging_current.clone().unwrap_or(5.0),
        }
    }
}

impl Recall for RecallBattery {
    type Source = Battery;

    fn remember(&mut self, source: &Battery) {
        self.voltage = Some(source.voltage);
        self.capacity = Some(source.capacity);
        self.charging_current = Some(source.charging_current);
    }
}

// TODO(@xiyuoh) Update RobotProperty trait to accommodate properties that can accommodate multiple Kinds
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct PowerDissipation {
    #[reflect(ignore)]
    pub config: Value,
}

impl Default for PowerDissipation {
    fn default() -> Self {
        Self {
            config: Value::Object(Map::new()),
        }
    }
}

#[cfg(feature = "bevy")]
impl RobotProperty for PowerDissipation {
    fn new(kind: String, config: Value) -> Self {
        let mut property_map = Map::new();
        property_map.insert(kind.clone(), config);

        Self {
            config: Value::Object(property_map),
        }
    }

    fn is_default(&self) -> bool {
        if *self == Self::default() {
            return true;
        }
        false
    }

    fn kind(&self) -> Option<String> {
        // This RobotProperty supports multiple kinds
        None
    }

    fn label() -> String {
        "Power Dissipation".to_string()
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallPowerDissipation {
    pub config: Option<Value>,
}

impl Recall for RecallPowerDissipation {
    type Source = PowerDissipation;

    fn remember(&mut self, source: &PowerDissipation) {
        self.config = Some(source.config.clone());
    }
}

// Supported kinds of PowerDissipation
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct MechanicalSystem {
    pub mass: f32,
    pub moment_of_inertia: f32,
    pub friction_coefficient: f32,
}

impl Default for MechanicalSystem {
    fn default() -> Self {
        Self {
            mass: 20.0,
            moment_of_inertia: 10.0,
            friction_coefficient: 0.22,
        }
    }
}

#[cfg(feature = "bevy")]
impl RobotPropertyKind for MechanicalSystem {
    fn label() -> String {
        "Mechanical System".to_string()
    }
}

#[cfg(feature = "bevy")]
impl From<&ElementMap> for MechanicalSystem {
    fn from(elements: &ElementMap) -> Self {
        let mut mechanical_system = MechanicalSystem::default();
        if let Some(mass) = elements
            .get("mass")
            .and_then(|mass| f64::try_from(mass.data.clone()).ok())
        {
            mechanical_system.mass = mass as f32;
        }
        if let Some(moment_of_inertia) = elements
            .get("inertia")
            .and_then(|moment_of_inertia| f64::try_from(moment_of_inertia.data.clone()).ok())
        {
            mechanical_system.moment_of_inertia = moment_of_inertia as f32;
        }
        if let Some(friction_coefficient) = elements
            .get("friction_coefficient")
            .and_then(|friction_coefficient| f64::try_from(friction_coefficient.data.clone()).ok())
        {
            mechanical_system.friction_coefficient = friction_coefficient as f32;
        }

        mechanical_system
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallMechanicalSystem {
    pub mass: Option<f32>,
    pub moment_of_inertia: Option<f32>,
    pub friction_coefficient: Option<f32>,
}

#[cfg(feature = "bevy")]
impl RecallPropertyKind for RecallMechanicalSystem {
    type Kind = MechanicalSystem;

    fn assume(&self) -> MechanicalSystem {
        MechanicalSystem {
            mass: self.mass.clone().unwrap_or(20.0),
            moment_of_inertia: self.moment_of_inertia.clone().unwrap_or(10.0),
            friction_coefficient: self.friction_coefficient.clone().unwrap_or(0.22),
        }
    }
}

impl Recall for RecallMechanicalSystem {
    type Source = MechanicalSystem;

    fn remember(&mut self, source: &MechanicalSystem) {
        self.mass = Some(source.mass);
        self.moment_of_inertia = Some(source.moment_of_inertia);
        self.friction_coefficient = Some(source.friction_coefficient);
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct AmbientSystem {
    pub idle_power: f32,
}

impl Default for AmbientSystem {
    fn default() -> Self {
        Self { idle_power: 20.0 }
    }
}

#[cfg(feature = "bevy")]
impl RobotPropertyKind for AmbientSystem {
    fn label() -> String {
        "Ambient System".to_string()
    }
}

#[cfg(feature = "bevy")]
impl From<&ElementMap> for AmbientSystem {
    fn from(elements: &ElementMap) -> Self {
        let mut ambient_system = AmbientSystem::default();
        if let Some(power) = elements
            .get("nominal_power")
            .and_then(|power| f64::try_from(power.data.clone()).ok())
        {
            ambient_system.idle_power = power as f32;
        }

        ambient_system
    }
}

#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallAmbientSystem {
    pub idle_power: Option<f32>,
}

#[cfg(feature = "bevy")]
impl RecallPropertyKind for RecallAmbientSystem {
    type Kind = AmbientSystem;

    fn assume(&self) -> AmbientSystem {
        AmbientSystem {
            idle_power: self.idle_power.clone().unwrap_or(20.0),
        }
    }
}

impl Recall for RecallAmbientSystem {
    type Source = AmbientSystem;

    fn remember(&mut self, source: &AmbientSystem) {
        self.idle_power = Some(source.idle_power);
    }
}
