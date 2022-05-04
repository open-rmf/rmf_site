use super::level_transform::LevelTransform;
use super::site_map::{Editable, Handles};
use super::vertex::Vertex;
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use bevy_mod_picking::PickableBundle;

#[derive(Component, Inspectable, Clone, Default)]
pub struct Lane {
    pub start: usize,
    pub end: usize,
}

impl Lane {
    pub fn spawn(
        &self,
        vertices: &Vec<Vertex>,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        handles: &Res<Handles>,
        transform: &LevelTransform,
    ) {
        let v1 = &vertices[self.start];
        let v2 = &vertices[self.end];
        let dx = v2.x_meters - v1.x_meters;
        let dy = v2.y_meters - v1.y_meters;
        let length = Vec2::from([dx as f32, dy as f32]).length();
        let width = 0.5 as f32;
        let yaw = dy.atan2(dx) as f32;
        let cx = ((v1.x_meters + v2.x_meters) / 2.) as f32;
        let cy = ((v1.y_meters + v2.y_meters) / 2.) as f32;

        commands
            .spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::from([length, width])))),
                material: handles.lane_material.clone(),
                transform: Transform {
                    translation: Vec3::new(cx, cy, 0.01 + transform.translation[2] as f32),
                    rotation: Quat::from_rotation_z(yaw),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert_bundle(PickableBundle::default())
            .insert(Editable::Lane(self.clone()));
    }

    pub fn from_yaml(value: &serde_yaml::Value) -> Lane {
        let data = value.as_sequence().unwrap();
        let start = data[0].as_u64().unwrap();
        let end = data[1].as_u64().unwrap();
        return Lane {
            start: start as usize,
            end: end as usize,
        };
    }
}
