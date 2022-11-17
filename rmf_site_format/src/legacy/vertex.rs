use super::rbmf::*;
use crate::{
    AssetSource, is_default, IsStatic, Location, LocationTag, LocationTags, Model, ModelMarker,
    NameInSite, Pose,
};
use glam::DVec2;
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
    #[serde(default, skip_serializing_if = "is_default")]
    pub dock_name: RbmfString,
    #[serde(default, skip_serializing_if = "is_default")]
    pub lift_cabin: RbmfString,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Vertex(
    pub f64,
    pub f64,
    pub f64,
    pub String,
    #[serde(default)] pub VertexProperties,
);

impl Vertex {
    pub fn to_vec(&self) -> DVec2 {
        DVec2::new(self.0, self.1)
    }

    pub fn make_location(&self, anchor: u32) -> Option<Location<u32>> {
        let mut tags = Vec::new();
        let me = &self.4;
        if me.is_charger.1 {
            tags.push(LocationTag::Charger);
        }

        if me.is_parking_spot.1 {
            tags.push(LocationTag::ParkingSpot);
        }

        if me.is_holding_point.1 {
            tags.push(LocationTag::HoldingPoint);
        }

        if !me.spawn_robot_name.is_empty() && !me.spawn_robot_type.is_empty() {
            tags.push(LocationTag::SpawnRobot(Model {
                name: NameInSite(me.spawn_robot_name.1.clone()),
                source: AssetSource::Search(me.spawn_robot_type.1.clone()),
                pose: Pose::default(),
                is_static: IsStatic(false),
                marker: ModelMarker,
            }))
        }

        if tags.is_empty() {
            return None;
        } else {
            return Some(Location {
                anchor: anchor.into(),
                tags: LocationTags(tags),
            });
        }
    }
}
