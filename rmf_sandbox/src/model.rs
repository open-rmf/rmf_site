use bevy::prelude::*;

#[derive(serde::Deserialize, Component, Clone, Default)]
pub struct Model {
    pub model_name: String,
    #[serde(rename = "name")]
    pub instance_name: String,
    #[serde(rename = "static")]
    pub static_: bool,
    pub x: f64,
    pub y: f64,
    #[serde(rename = "z")]
    pub z_offset: f64,
    pub yaw: f64,
}

impl Model {
    pub fn transform(&self) -> Transform {
        Transform {
            rotation: Quat::from_rotation_z((self.yaw - 1.5707) as f32),
            translation: Vec3::new(self.x as f32, self.y as f32, self.z_offset as f32),
            scale: Vec3::ONE,
        }
    }

    pub fn from_xyz_yaw(
        instance_name: &str,
        model_name: &str,
        x: f64,
        y: f64,
        z: f64,
        yaw: f64,
    ) -> Model {
        return Model {
            instance_name: instance_name.to_string(),
            model_name: model_name.to_string(),
            x,
            y,
            yaw: yaw,
            z_offset: z,
            static_: true,
        };
    }
}
