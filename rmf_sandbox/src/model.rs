use super::level_transform::LevelTransform;
//use super::site_map::Handles;
//use super::site_map::{Editable, Handles};
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
        //meshes: &mut ResMut<Assets<Mesh>>,
        //handles: &Res<Handles>,
        _transform: &LevelTransform,
        asset_server: &Res<AssetServer>,
    ) {
        // TODO: need to set up https on this server, for this WASM to work
        // when hosted over https
        #[cfg(not(target_arch = "wasm32"))]
        {
            //let bundle_path = String::from("http://models.sandbox.open-rmf.org/models/")
            //let bundle_path = String::from("")
            let bundle_path = String::from("sandbox://")
                + &self.model_name
                + &String::from(".glb#Scene0");
            /*
            println!(
                "spawning {} at {}, {}",
                &bundle_path, self.x_meters, self.y_meters
            );
            */
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
    }

    pub fn from_yaml(value: &serde_yaml::Value) -> Model {
        let x_raw = value["x"].as_f64().unwrap();
        let y_raw = value["y"].as_f64().unwrap();
        let yaw = value["yaw"].as_f64().unwrap() - 3.14159 / 2.;
        let model_name = value["model_name"].as_str().unwrap();
        let instance_name = value["name"].as_str().unwrap();
        // println!("model {} at ({}, {})", model_name, x_raw, y_raw);
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
