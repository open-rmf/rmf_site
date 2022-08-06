use super::rbmf::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct FloorParameters {
    pub texture_name: RbmfString,
    pub texture_rotation: RbmfFloat,
    pub texture_scale: RbmfFloat,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Floor {
    pub parameters: FloorParameters,
    pub vertices: Vec<usize>,
}
