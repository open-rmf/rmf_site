use crate::rbmf::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct FloorParameters {
    texture_name: RbmfString,
    texture_rotation: RbmfFloat,
    texture_scale: RbmfFloat,
}

#[derive(Deserialize, Serialize, Clone, Component, Default)]
pub struct Floor {
    parameters: FloorParameters,
    vertices: Vec<usize>,
}
