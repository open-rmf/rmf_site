use super::vertex::Vertex;
use crate::rbmf::*;
use bevy::prelude::*;
use bevy::render::mesh::{Indices, PrimitiveTopology};
use serde::{Deserialize, Serialize};

fn default_height() -> RbmfFloat {
    RbmfFloat::from(2.)
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WallProperties {
    pub alpha: RbmfFloat,
    pub texture_name: RbmfString,
    #[serde(default = "default_height")]
    pub texture_height: RbmfFloat,
}

impl Default for WallProperties {
    fn default() -> Self {
        Self {
            alpha: RbmfFloat::default(),
            texture_name: RbmfString::from("default".to_string()),
            texture_height: default_height(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Component, Default)]
pub struct Wall(pub usize, pub usize, pub WallProperties, pub f32);

impl Wall {
    pub fn mesh(&self, v1: &Vertex, v2: &Vertex, height: f32) -> Mesh {
        let dx = (v2.0 - v1.0) as f32;
        let dy = (v2.1 - v1.1) as f32;
        let length = Vec2::from([dx, dy]).length();
        let width = 0.1 as f32;

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
        mesh
    }

    pub fn transform(&self, v1: &Vertex, v2: &Vertex) -> Transform {
        let dx = (v2.0 - v1.0) as f32;
        let dy = (v2.1 - v1.1) as f32;
        let yaw = dy.atan2(dx) as f32;
        let cx = ((v1.0 + v2.0) / 2.) as f32;
        let cy = ((v1.1 + v2.1) / 2.) as f32;

        Transform {
            translation: Vec3::new(cx, cy, 0.),
            // base length is 20, base height is 3
            scale: Vec3::new(1., 1., (*self.2.texture_height / 3.) as f32),
            rotation: Quat::from_rotation_z(yaw),
            ..Default::default()
        }
    }
}
