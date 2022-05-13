use super::vertex::Vertex;
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;

#[derive(serde::Deserialize, Component, Inspectable, Clone, Default)]
#[serde(from = "MeasurementRaw")]
pub struct Measurement {
    pub start: usize,
    pub end: usize,
    pub distance: f64,
}

impl From<MeasurementRaw> for Measurement {
    fn from(raw: MeasurementRaw) -> Measurement {
        Measurement {
            start: raw.data.0,
            end: raw.data.1,
            distance: raw.data.2.distance.1,
        }
    }
}

impl Measurement {
    pub fn transform(&self, v1: &Vertex, v2: &Vertex) -> Transform {
        let dx = v2.x_meters - v1.x_meters;
        let dy = v2.y_meters - v1.y_meters;
        let length = Vec2::from([dx as f32, dy as f32]).length();
        let width = 0.25 as f32;
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
struct MeasurementRaw {
    data: (usize, usize, MeasurementProperties),
}

#[derive(serde::Deserialize)]
struct MeasurementProperties {
    distance: (f64, f64),
}
