use super::rbmf::*;
use crate::{
    AssociatedGraphs, Location, LocationTag, LocationTags, NameInSite, is_default,
    legacy::model::Model,
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
    pub mutex: Option<RbmfString>,
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

        let name = if self.3.is_empty() {
            None
        } else {
            Some(self.3.clone())
        };

        let has_mutex_group = self.4.mutex.as_ref().is_some_and(|n| !n.is_empty());
        if tags.is_empty() && name.is_none() && !has_mutex_group {
            return None;
        } else {
            return Some(Location {
                anchor: anchor.into(),
                tags: LocationTags(tags),
                name: NameInSite(name.unwrap_or_default()),
                mutex: Default::default(),
                graphs: AssociatedGraphs::All,
            });
        }
    }

    pub fn spawn_robot(&self, anchor: u32) -> Option<Model> {
        let me = &self.4;
        if !me.spawn_robot_name.is_empty() && !me.spawn_robot_type.is_empty() {
            return Some(Model {
                model_name: me.spawn_robot_type.1.clone(),
                instance_name: me.spawn_robot_name.1.clone(),
                static_: false,
                x: self.0,
                y: self.1,
                yaw: self.2,
                location: Some(anchor),
                ..Default::default()
            });
        }
        None
    }
}
