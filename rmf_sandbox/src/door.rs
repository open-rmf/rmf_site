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
