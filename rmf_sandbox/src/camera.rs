use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Component, Clone)]
pub struct Camera {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub pitch: f64,
    pub yaw: f64,
}

impl Default for Camera {
    fn default() -> Self {
        Self {
            name: "Camera1".to_string(),
            x: 0.0,
            y: 0.0,
            z: 0.0,
            pitch: 0.0,
            yaw: 0.0,
        }
    }
}
