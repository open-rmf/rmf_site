use bevy::prelude::*;

// For now, this is just (scale, yaw, translate). In the future, we may need
// nonlinear warping as well, so this is in a custom class, not using Bevy's
// transform modules.

#[derive(Component, Clone, Default)]
pub struct LevelTransform {
    pub yaw: f64,
    pub translation: [f64; 3],
}

impl LevelTransform {
    /*
    pub fn from_yaw_translation(yaw: f64, x: f64, y: f64, z: f64) -> LevelTransform {
        LevelTransform {
            translation: [x, y, z],
            yaw: yaw,
        }
    }
    */
}
