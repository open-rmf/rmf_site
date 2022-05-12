use super::vertex::Vertex;
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;

#[derive(serde::Deserialize, Component, Inspectable, Clone, Default)]
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
    pub fn transform(&self, vertices: &Vec<Vertex>) -> Transform {
        let v1 = &vertices[self.start];
        let v2 = &vertices[self.end];
        let dx = v2.x_meters - v1.x_meters;
        let dy = v2.y_meters - v1.y_meters;
        let length = Vec2::from([dx as f32, dy as f32]).length();
        let width = 0.5 as f32;
        let yaw = dy.atan2(dx) as f32;
        let cx = ((v1.x_meters + v2.x_meters) / 2.) as f32;
        let cy = ((v1.y_meters + v2.y_meters) / 2.) as f32;
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
