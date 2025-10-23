use super::rbmf::*;
use crate::is_default;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct LaneProperties {
    pub bidirectional: RbmfBool,
    pub graph_idx: RbmfInt,
    pub orientation: RbmfString,
    #[serde(default, skip_serializing_if = "is_default")]
    pub mutex: RbmfString,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Lane(pub usize, pub usize, pub LaneProperties);

pub const PASSIVE_LANE_HEIGHT: f32 = 0.001;
pub const SELECTED_LANE_HEIGHT: f32 = 0.002;
pub const HOVERED_LANE_HEIGHT: f32 = 0.003;
pub const LANE_WIDTH: f32 = 0.5;
