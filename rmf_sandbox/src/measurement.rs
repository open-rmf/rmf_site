use super::vertex::Vertex;
use bevy::prelude::*;

#[derive(serde::Deserialize, Component, Clone, Default)]
#[serde(from = "MeasurementRaw")]
pub struct Measurement {
    pub start: usize,
    pub end: usize,
    // TODO: For new cartesian format, there should be no need for this value since the
    // metric distance is always equal to the distance between start and end.
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
        let dx = v2.x - v1.x;
        let dy = v2.y - v1.y;
        let length = Vec2::from([dx as f32, dy as f32]).length();
        let width = 0.25 as f32;
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
struct MeasurementRaw {
    data: (usize, usize, MeasurementProperties),
}

#[derive(serde::Deserialize)]
struct MeasurementProperties {
    distance: (f64, f64),
}
