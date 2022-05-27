use super::vertex::Vertex;
use crate::rbmf::*;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Component, Clone, Default)]
#[serde(from = "MeasurementRaw", into = "MeasurementRaw")]
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
            start: raw.0,
            end: raw.1,
            distance: raw.2.distance.1,
        }
    }
}

impl Into<MeasurementRaw> for Measurement {
    fn into(self) -> MeasurementRaw {
        MeasurementRaw(
            self.start,
            self.end,
            MeasurementProperties {
                distance: RbmfFloat::from(self.distance),
            },
        )
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

#[derive(Deserialize, Serialize)]
struct MeasurementRaw(usize, usize, MeasurementProperties);

#[derive(Deserialize, Serialize)]
struct MeasurementProperties {
    distance: RbmfFloat,
}
