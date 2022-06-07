use std::fmt::Display;

use crate::rbmf::*;
use bevy::prelude::Component;
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

#[derive(Deserialize, Serialize, Clone, Component)]
pub struct Door(pub usize, pub usize, pub DoorProperties);

pub enum DoorType {
    SingleSliding = 1,
    DoubleSliding = 2,
    SingleTelescope = 3,
    DoubleTelescope = 4,
    SingleSwing = 5,
    DoubleSwing = 6,
}

impl Display for DoorType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            DoorType::SingleSliding => write!(f, "Single Sliding"),
            DoorType::DoubleSliding => write!(f, "Double Sliding"),
            DoorType::SingleTelescope => write!(f, "Single Telescope"),
            DoorType::DoubleTelescope => write!(f, "Double Telescope"),
            DoorType::SingleSwing => write!(f, "Single Swing"),
            DoorType::DoubleSwing => write!(f, "Double Swing"),
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
];
