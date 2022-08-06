use super::rbmf::*;
use crate::is_default;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct VertexProperties {
    #[serde(default, skip_serializing_if = "is_default")]
    pub is_charger: RbmfBool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub is_parking_spot: RbmfBool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub is_holding_point: RbmfBool,
    #[serde(default, skip_serializing_if = "is_default")]
    pub spawn_robot_name: RbmfString,
    #[serde(default, skip_serializing_if = "is_default")]
    pub spawn_robot_type: RbmfString,
    #[serde(default, skip_serializing_if = "is_default")]
    pub dropoff_ingestor: RbmfString,
    #[serde(default, skip_serializing_if = "is_default")]
    pub pickup_dispenser: RbmfString,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Vertex(
    pub f64,
    pub f64,
    pub f64,
    pub String,
    #[serde(default)] pub VertexProperties,
);
