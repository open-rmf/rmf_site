use super::vertex::Vertex;
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;

#[derive(serde::Deserialize, Component, Inspectable, Clone, Default)]
#[serde(from = "WallRaw")]
pub struct Wall {
    pub start: usize,
    pub end: usize,
}

impl From<WallRaw> for Wall {
    fn from(raw: WallRaw) -> Wall {
        Wall {
            start: raw.data.0,
            end: raw.data.1,
        }
    }
}

impl Wall {
    pub fn transform(&self, vertices: &Vec<Vertex>) -> Transform {
        let v1 = &vertices[self.start];
        let v2 = &vertices[self.end];
        let dx = (v2.x_meters - v1.x_meters) as f32;
        let dy = (v2.y_meters - v1.y_meters) as f32;
        let length = Vec2::from([dx, dy]).length();
        let width = 0.1 as f32;
        let height = 2.0 as f32;
        let yaw = dy.atan2(dx) as f32;
        let cx = ((v1.x_meters + v2.x_meters) / 2.) as f32;
        let cy = ((v1.y_meters + v2.y_meters) / 2.) as f32;

        Transform {
            translation: Vec3::new(cx, cy, height / 2.),
            rotation: Quat::from_rotation_z(yaw),
            scale: Vec3::new(length, width, height),
            ..Default::default()
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(transparent)]
struct WallRaw {
    data: (usize, usize, WallProperties),
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct WallProperties {
    alpha: (usize, usize),
    texture_name: (usize, String),
}
