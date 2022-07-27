use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Component, Clone, Default)]
pub struct Camera {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub pitch: f64,
    pub yaw: f64,
}

impl Camera {
    pub fn transform(&self) -> Transform {
        Transform {
            translation: Vec3::new(self.x as f32, self.y as f32, self.z as f32),
            //EulerRot::ZYX means apply yaw, pitch, roll in that order
            rotation: Quat::from_euler(
                EulerRot::ZYX,
                self.yaw as f32,
                self.pitch as f32,
                0.0 as f32,
            ),
            scale: Vec3::new(0.1, 0.1, 0.1),
            ..Default::default()
        }
    }
}
