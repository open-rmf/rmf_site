use std::fmt::Display;

use super::rbmf::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct DoorProperties {
    pub right_left_ratio: RbmfFloat,
    pub motion_axis: RbmfString,
    pub motion_degrees: RbmfFloat,
    pub motion_direction: RbmfInt,
    pub name: RbmfString,
    pub plugin: RbmfString,
    #[serde(rename = "type")]
    pub type_: RbmfString,
}

impl Default for DoorProperties {
    fn default() -> Self {
        Self {
            right_left_ratio: RbmfFloat::from(1.),
            motion_axis: RbmfString::from("start"),
            motion_degrees: RbmfFloat::from(90.),
            motion_direction: RbmfInt::from(1),
            name: RbmfString::from("new_door"),
            plugin: RbmfString::from("normal"),
            type_: RbmfString::from("hinged"),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Door(pub usize, pub usize, pub DoorProperties);

pub enum DoorType {
    Unknown,
    SingleSliding,
    DoubleSliding,
    SingleTelescope,
    DoubleTelescope,
    SingleSwing,
    DoubleSwing,
    SingleHinged,
    DoubleHinged,
}

impl DoorType {
    pub fn to_value(&self) -> String {
        match self {
            DoorType::Unknown => "unknown".to_string(),
            DoorType::SingleSliding => "sliding".to_string(),
            DoorType::DoubleSliding => "double_sliding".to_string(),
            DoorType::SingleTelescope => "telescope".to_string(),
            DoorType::DoubleTelescope => "double_telescope".to_string(),
            DoorType::SingleSwing => "swing".to_string(),
            DoorType::DoubleSwing => "double_swing".to_string(),
            DoorType::SingleHinged => "hinged".to_string(),
            DoorType::DoubleHinged => "double_hinged".to_string(),
        }
    }
}

impl From<&str> for DoorType {
    fn from(s: &str) -> Self {
        match s {
            "sliding" => DoorType::SingleSliding,
            "double_sliding" => DoorType::DoubleSliding,
            "telescope" => DoorType::SingleTelescope,
            "double_telescope" => DoorType::DoubleTelescope,
            "swing" => DoorType::SingleSwing,
            "double_swing" => DoorType::DoubleSwing,
            "hinged" => DoorType::SingleHinged,
            "double_hinged" => DoorType::DoubleHinged,
            _ => DoorType::Unknown,
        }
    }
}

impl Display for DoorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoorType::Unknown => write!(f, "Unknown"),
            DoorType::SingleSliding => write!(f, "Sliding"),
            DoorType::DoubleSliding => write!(f, "Double Sliding"),
            DoorType::SingleTelescope => write!(f, "Telescope"),
            DoorType::DoubleTelescope => write!(f, "Double Telescope"),
            DoorType::SingleSwing => write!(f, "Swing"),
            DoorType::DoubleSwing => write!(f, "Double Swing"),
            DoorType::SingleHinged => write!(f, "Hinged"),
            DoorType::DoubleHinged => write!(f, "Double Hinged"),
        }
    }
}

pub static DOOR_TYPES: &[DoorType] = &[
    DoorType::SingleSliding,
    DoorType::DoubleSliding,
    DoorType::SingleTelescope,
    DoorType::DoubleTelescope,
    DoorType::SingleSwing,
    DoorType::DoubleSwing,
    DoorType::SingleHinged,
    DoorType::DoubleHinged,
];
