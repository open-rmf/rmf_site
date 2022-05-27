use crate::rbmf::*;
use crate::utils::is_option_default;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Component, Clone, Default)]
#[serde(try_from = "VertexRaw", into = "VertexRaw")]
pub struct Vertex {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub is_charger: bool,
    pub is_holding_point: bool,
    pub is_parking_spot: bool,
    pub spawn_robot_name: String,
    pub spawn_robot_type: String,
    pub dropoff_ingestor: String,
    pub pickup_dispenser: String,
}

impl TryFrom<VertexRaw> for Vertex {
    type Error = String;

    /// NOTE: This loads the vertex data "as is", in older maps, it will contain the raw
    /// "pixel coordinates" which needs to be converted to meters for the site map viewer
    /// to work correctly.
    fn try_from(raw: VertexRaw) -> Result<Vertex, Self::Error> {
        Ok(Vertex {
            x: raw.0,
            y: raw.1,
            z: raw.2,
            name: raw.3,
            is_charger: raw.4.is_charger.map_or(false, |x| x.1),
            is_holding_point: raw.4.is_holding_point.map_or(false, |x| x.1),
            is_parking_spot: raw.4.is_parking_spot.map_or(false, |x| x.1),
            spawn_robot_name: raw.4.spawn_robot_name.map_or("".to_string(), |x| x.1),
            spawn_robot_type: raw.4.spawn_robot_type.map_or("".to_string(), |x| x.1),
            dropoff_ingestor: raw.4.dropoff_ingestor.map_or("".to_string(), |x| x.1),
            pickup_dispenser: raw.4.pickup_dispenser.map_or("".to_string(), |x| x.1),
        })
    }
}

impl Into<VertexRaw> for Vertex {
    fn into(self) -> VertexRaw {
        VertexRaw(
            self.x,
            self.y,
            self.z,
            self.name,
            VertexProperties {
                is_charger: Some(RbmfBool::from(self.is_charger)),
                is_holding_point: Some(RbmfBool::from(self.is_holding_point)),
                is_parking_spot: Some(RbmfBool::from(self.is_parking_spot)),
                spawn_robot_name: Some(RbmfString::from(self.spawn_robot_name)),
                spawn_robot_type: Some(RbmfString::from(self.spawn_robot_type)),
                dropoff_ingestor: Some(RbmfString::from(self.dropoff_ingestor)),
                pickup_dispenser: Some(RbmfString::from(self.pickup_dispenser)),
            },
        )
    }
}

impl Vertex {
    pub fn transform(&self) -> Transform {
        Transform {
            translation: Vec3::new(self.x as f32, self.y as f32, 0.),
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Serialize, Default)]
struct VertexProperties {
    #[serde(skip_serializing_if = "is_option_default")]
    is_charger: Option<RbmfBool>,
    #[serde(skip_serializing_if = "is_option_default")]
    is_parking_spot: Option<RbmfBool>,
    #[serde(skip_serializing_if = "is_option_default")]
    is_holding_point: Option<RbmfBool>,
    #[serde(skip_serializing_if = "is_option_default")]
    spawn_robot_name: Option<RbmfString>,
    #[serde(skip_serializing_if = "is_option_default")]
    spawn_robot_type: Option<RbmfString>,
    #[serde(skip_serializing_if = "is_option_default")]
    dropoff_ingestor: Option<RbmfString>,
    #[serde(skip_serializing_if = "is_option_default")]
    pickup_dispenser: Option<RbmfString>,
}

#[derive(Deserialize, Serialize)]
struct VertexRaw(f64, f64, f64, String, #[serde(default)] VertexProperties);
