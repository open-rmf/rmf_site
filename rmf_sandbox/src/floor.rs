use crate::rbmf::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize)]
pub struct FloorParameters {
    texture_name: RbmfString,
    texture_rotation: RbmfFloat,
    texture_scale: RbmfFloat,
    vertices: Vec<usize>,
}

#[derive(Deserialize, Serialize)]
pub struct Floor {
    parameters: FloorParameters,
}
