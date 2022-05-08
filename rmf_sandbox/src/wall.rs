use super::level_transform::LevelTransform;
use super::site_map::{Editable, Handles};
use super::vertex::Vertex;
use bevy::prelude::*;
use bevy_inspector_egui::Inspectable;
use bevy_mod_picking::PickableBundle;
use bevy::render::mesh::{Indices, PrimitiveTopology};

#[derive(Component, Inspectable, Clone, Default)]
pub struct Wall {
    pub start: usize,
    pub end: usize,
    pub texture_name: String,
    pub height: f32,
}

impl Wall {
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
        let dx = (v2.x_meters - v1.x_meters) as f32;
        let dy = (v2.y_meters - v1.y_meters) as f32;
        let length = Vec2::from([dx, dy]).length();
        let width = 0.1 as f32;
        let height = 3.0 as f32;
        let yaw = dy.atan2(dx) as f32;
        let cx = ((v1.x_meters + v2.x_meters) / 2.) as f32;
        let cy = ((v1.y_meters + v2.y_meters) / 2.) as f32;

        // let mut mesh = Mesh::new(PrimitiveTopology::
        // we need to wrap the base wall texture around the wall mesh
        // differently from the way the standard "box" mesh helper does,
        // so we'll craft our own meshes here, copying and tweaking the
        // source of From<Box>::from in bevy_render/src/mesh/shape/mod.rs
        let min_x = -length / (2. as f32);
        let max_x = length / (2. as f32);
        let min_y = -width / (2. as f32);
        let max_y = width / (2. as f32);

        let v = &[
            // Top
            ([min_x, min_y, height], [0., 0., 1.0], [1.0, 0.]),
            ([max_x, min_y, height], [0., 0., 1.0], [1.0, 0.]),
            ([max_x, max_y, height], [0., 0., 1.0], [1.0, 0.]),
            ([min_x, max_y, height], [0., 0., 1.0], [1.0, 0.]),
            // Bottom
            ([min_x, max_y, 0.], [0., 0., -1.0], [0., 1.0]),
            ([max_x, max_y, 0.], [0., 0., -1.0], [0., 1.0]),
            ([max_x, min_y, 0.], [0., 0., -1.0], [0., 1.0]),
            ([min_x, min_y, 0.], [0., 0., -1.0], [0., 1.0]),
            // Right
            ([max_x, min_y, 0.], [1.0, 0., 0.], [0., 1.0]),
            ([max_x, max_y, 0.], [1.0, 0., 0.], [1.0, 1.0]),
            ([max_x, max_y, height], [1.0, 0., 0.], [1.0, 0.]),
            ([max_x, min_y, height], [1.0, 0., 0.], [0., 0.]),
            // Left
            ([min_x, min_y, height], [-1.0, 0., 0.], [1.0, 0.]),
            ([min_x, max_y, height], [-1.0, 0., 0.], [0., 0.]),
            ([min_x, max_y, 0.], [-1.0, 0., 0.], [0., 1.0]),
            ([min_x, min_y, 0.], [-1.0, 0., 0.], [1.0, 1.0]),
            // Front
            ([max_x, max_y, 0.], [0., 1.0, 0.], [1.0, 1.0]),
            ([min_x, max_y, 0.], [0., 1.0, 0.], [0., 1.0]),
            ([min_x, max_y, height], [0., 1.0, 0.], [0., 0.]),
            ([max_x, max_y, height], [0., 1.0, 0.], [1.0, 0.]),
            // Back
            ([max_x, min_y, height], [0., -1.0, 0.], [0., 0.]),
            ([min_x, min_y, height], [0., -1.0, 0.], [1.0, 0.]),
            ([min_x, min_y, 0.], [0., -1.0, 0.], [1.0, 1.0]),
            ([max_x, min_y, 0.], [0., -1.0, 0.], [0., 1.0]),
        ];

        let mut positions = Vec::with_capacity(24);
        let mut normals = Vec::with_capacity(24);
        let mut uvs = Vec::with_capacity(24);

        for (position, normal, uv) in v.iter() {
            positions.push(*position);
            normals.push(*normal);
            uvs.push(*uv);
        }

        let indices = Indices::U32(vec![
            0, 1, 2, 2, 3, 0, // top
            4, 5, 6, 6, 7, 4, // bottom
            8, 9, 10, 10, 11, 8, // right
            12, 13, 14, 14, 15, 12, // left
            16, 17, 18, 18, 19, 16, // front
            20, 21, 22, 22, 23, 20, // back
        ]);

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(indices));

        commands
            .spawn_bundle(PbrBundle {
                //mesh: meshes.add(Mesh::from(shape::Box::new(length, width, height))),
                mesh: meshes.add(mesh),
                material: handles.wall_material.clone(),
                transform: Transform {
                    translation: Vec3::new(cx, cy, transform.translation[2] as f32),
                    rotation: Quat::from_rotation_z(yaw),
                    ..Default::default()
                },
                ..Default::default()
            })
            .insert_bundle(PickableBundle::default())
            .insert(Editable::Wall(self.clone()));
    }

    pub fn from_yaml(value: &serde_yaml::Value) -> Wall {
        let data = value.as_sequence().unwrap();
        let start = data[0].as_u64().unwrap();
        let end = data[1].as_u64().unwrap();

        let height = match data[2]["texture_height"].as_sequence() {
            Some(h) => h[1].as_f64().unwrap(),
            None => 2.0,
        };

        let texture_name = match data[2]["texture_name"].as_sequence() {
            Some(name) => name[1].as_str().unwrap().to_string(),
            None => "".to_string(),
        };

        return Wall {
            start: start as usize,
            end: end as usize,
            height: height as f32,
            texture_name: texture_name,
        };
    }
}
