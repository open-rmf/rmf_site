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
    ) {
        let v1 = &vertices[self.start];
        let v2 = &vertices[self.end];
        let dx = v2.x - v1.x;
        let dy = v2.y - v1.y;
        let length = Vec2::from([dx as f32, dy as f32]).length();
        let width = 0.5 as f32;
        let yaw = dy.atan2(dx) as f32;
        let cx = ((v1.x + v2.x) / 2.) as f32;
        let cy = ((v1.y + v2.y) / 2.) as f32;

        commands
            .spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::from([length, width])))),
                material: handles.lane_material.clone(),
                transform: Transform {
                    translation: Vec3::new(cx, cy, 0.01),
                    rotation: Quat::from_rotation_z(yaw),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert_bundle(PickableBundle::default())
            .insert(Editable::Lane(self.clone()));
    }
}
