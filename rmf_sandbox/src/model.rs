use super::level_transform::LevelTransform;
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
//use bevy_mod_picking::PickableBundle;
use serde_yaml;

#[derive(Component, Inspectable, Clone, Default)]
pub struct Model {
    pub x_raw: f64,
    pub y_raw: f64,
    pub yaw: f64,
    pub x_meters: f64,
    pub y_meters: f64,
    pub instance_name: String,
    pub model_name: String,
}

impl Model {
    pub fn spawn(
        &self,
        commands: &mut Commands,
        _transform: &LevelTransform,
        asset_server: &Res<AssetServer>,
    ) {
        let bundle_path =
            String::from("sandbox://") + &self.model_name + &String::from(".glb#Scene0");
        let glb = asset_server.load(&bundle_path);
        commands
            .spawn_bundle((
                Transform {
                    rotation: Quat::from_rotation_z(self.yaw as f32),
                    translation: Vec3::new(self.x_meters as f32, self.y_meters as f32, 0.),
                    scale: Vec3::ONE,
                },
                GlobalTransform::identity(),
            ))
            .with_children(|parent| {
                parent.spawn_scene(glb);
            });
    }

    pub fn from_yaml(value: &serde_yaml::Value) -> Model {
        let x_raw = value["x"].as_f64().unwrap();
        let y_raw = value["y"].as_f64().unwrap();
        let yaw = value["yaw"].as_f64().unwrap() - 3.14159 / 2.;
        let model_name = value["model_name"].as_str().unwrap();
        let instance_name = value["name"].as_str().unwrap();
        return Model {
            x_raw: x_raw,
            y_raw: -y_raw,
            x_meters: x_raw,
            y_meters: -y_raw,
            yaw: yaw,
            model_name: model_name.to_string(),
            instance_name: instance_name.to_string(),
        };
    }
}
