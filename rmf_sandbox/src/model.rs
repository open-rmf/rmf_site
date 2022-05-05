use super::level_transform::LevelTransform;
use super::site_map::{Editable, Handles};
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use bevy_mod_picking::PickableBundle;
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
        meshes: &mut ResMut<Assets<Mesh>>,
        handles: &Res<Handles>,
        transform: &LevelTransform,
    ) {
        /*
        commands
            .spawn_bundle(PbrBundle {
                mesh: handles.vertex_mesh.clone(),
                material: handles.vertex_material.clone(),
                transform: Transform {
                    translation: Vec3::new(
                        self.x_meters as f32,
                        self.y_meters as f32,
                        transform.translation[2] as f32,
                    ),
                    rotation: Quat::from_rotation_x(1.57),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert_bundle(PickableBundle::default())
            .insert(Editable::Vertex(self.clone()));
        */
    }

    pub fn from_yaml(value: &serde_yaml::Value) -> Model {
        let x_raw = value["x"].as_f64().unwrap();
        let y_raw = value["y"].as_f64().unwrap();
        let yaw = value["yaw"].as_f64().unwrap();
        let model_name = value["model_name"].as_str().unwrap();
        let instance_name = value["name"].as_str().unwrap();
        println!("model {} at ({}, {})", model_name, x_raw, y_raw);
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
