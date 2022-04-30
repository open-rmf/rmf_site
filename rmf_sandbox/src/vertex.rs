use super::site_map::{Editable, Handles};
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use bevy_mod_picking::PickableBundle;
use serde_yaml;

#[derive(Component, Inspectable, Clone, Default)]
pub struct Vertex {
    pub x: f64,
    pub y: f64,
    pub _name: String,
}

impl Vertex {
    pub fn spawn(&self, commands: &mut Commands, handles: &Res<Handles>) {
        commands
            .spawn_bundle(PbrBundle {
                mesh: handles.vertex_mesh.clone(),
                material: handles.vertex_material.clone(),
                transform: Transform {
                    translation: Vec3::new(self.x as f32, self.y as f32, 0.0),
                    rotation: Quat::from_rotation_x(1.57),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert_bundle(PickableBundle::default())
            .insert(Editable::Vertex(self.clone()));
    }

    pub fn from_yaml(value: &serde_yaml::Value) -> Vertex {
        let data = value.as_sequence().unwrap();
        let x = data[0].as_f64().unwrap();
        let y = data[1].as_f64().unwrap();
        let name = if data.len() > 3 {
            data[3].as_str().unwrap().to_string()
        } else {
            String::new()
        };
        return Vertex {
            x: x,
            y: -y,
            _name: name,
        };
    }
}
