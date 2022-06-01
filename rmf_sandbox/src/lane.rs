use super::vertex::Vertex;
use crate::rbmf::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct LaneProperties {
    pub bidirectional: RbmfBool,
    pub graph_idx: RbmfInt,
    pub orientation: RbmfString,
}

#[derive(Deserialize, Serialize, Clone, Component)]
pub struct Lane(pub usize, pub usize, pub LaneProperties);

impl Lane {
    pub fn transform(&self, v1: &Vertex, v2: &Vertex) -> Transform {
        let v1 = v1;
        let v2 = v2;
        let dx = v2.0 - v1.0;
        let dy = v2.1 - v1.1;
        let length = Vec2::from([dx as f32, dy as f32]).length();
        let width = 0.5 as f32;
        let yaw = dy.atan2(dx) as f32;
        let cx = ((v1.0 + v2.0) / 2.) as f32;
        let cy = ((v1.1 + v2.1) / 2.) as f32;
        Transform {
            translation: Vec3::new(cx, cy, 0.01),
            rotation: Quat::from_rotation_z(yaw),
            scale: Vec3::new(length, width, 1.),
            ..Default::default()
        }
    }
}
