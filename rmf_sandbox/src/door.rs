use crate::rbmf::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct DoorProperties {
    right_left_ratio: RbmfFloat,
    motion_axis: RbmfString,
    motion_degrees: RbmfFloat,
    motion_direction: RbmfInt,
    name: RbmfString,
    plugin: RbmfString,
    #[serde(rename = "type")]
    type_: RbmfString,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct Door(usize, usize, DoorProperties);
