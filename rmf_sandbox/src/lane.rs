use super::vertex::Vertex;
use bevy::prelude::*;

#[derive(serde::Deserialize, Component, Clone, Default)]
#[serde(from = "LaneRaw")]
pub struct Lane {
    pub start: usize,
    pub end: usize,
}

impl From<LaneRaw> for Lane {
    fn from(raw: LaneRaw) -> Lane {
        Lane {
            start: raw.data.0,
            end: raw.data.1,
        }
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

#[derive(serde::Deserialize)]
#[serde(transparent)]
struct LaneRaw {
    // data: Vec<serde_yaml::Value>,
    data: (usize, usize, LaneProperties),
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct LaneProperties {
    bidirectional: (usize, bool),
    graph_idx: (usize, usize),
    orientation: (usize, String),
}
