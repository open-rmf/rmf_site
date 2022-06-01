use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Component, Clone, Default)]
pub struct Light {
    pub x: f64,
    pub y: f64,
    pub z_offset: f64,
    pub intensity: f64,
    pub range: f64,
}

impl Light {
    pub fn transform(&self) -> Transform {
        Transform {
            translation: Vec3::new(self.x as f32, self.y as f32, self.z_offset as f32),
            ..Default::default()
        }
    }
}
