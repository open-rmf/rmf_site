use super::vertex::Vertex;
use crate::rbmf::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Component, Clone, Default)]
#[serde(from = "LaneRaw", into = "LaneRaw")]
pub struct Lane {
    pub start: usize,
    pub end: usize,
    pub bidirectional: bool,
    pub graph_idx: i64,
    pub orientation: String,
}

impl From<LaneRaw> for Lane {
    fn from(raw: LaneRaw) -> Lane {
        Lane {
            start: raw.0,
            end: raw.1,
            bidirectional: raw.2.bidirectional.1,
            graph_idx: raw.2.graph_idx.1,
            orientation: raw.2.orientation.1,
        }
    }
}

impl Into<LaneRaw> for Lane {
    fn into(self) -> LaneRaw {
        LaneRaw(
            self.start,
            self.end,
            LaneProperties {
                bidirectional: RbmfBool::from(self.bidirectional),
                graph_idx: RbmfInt::from(self.graph_idx),
                orientation: RbmfString::from(self.orientation),
            },
        )
    }
}

impl Lane {
    pub fn transform(&self, v1: &Vertex, v2: &Vertex) -> Transform {
        let v1 = v1;
        let v2 = v2;
        let dx = v2.x - v1.x;
        let dy = v2.y - v1.y;
        let length = Vec2::from([dx as f32, dy as f32]).length();
        let width = 0.5 as f32;
        let yaw = dy.atan2(dx) as f32;
        let cx = ((v1.x + v2.x) / 2.) as f32;
        let cy = ((v1.y + v2.y) / 2.) as f32;
        Transform {
            translation: Vec3::new(cx, cy, 0.01),
            rotation: Quat::from_rotation_z(yaw),
            scale: Vec3::new(length, width, 1.),
            ..Default::default()
        }
    }
}

#[derive(Deserialize, Serialize)]
struct LaneRaw(usize, usize, LaneProperties);

#[derive(Deserialize, Serialize)]
struct LaneProperties {
    bidirectional: RbmfBool,
    graph_idx: RbmfInt,
    orientation: RbmfString,
}
