use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use bevy::render::mesh::Indices;
use bevy::render::mesh::PrimitiveTopology;
use std::f32::consts::FRAC_1_SQRT_2;

#[derive(Deserialize, Serialize, Component, Clone, Default)]
pub struct PhysicalCamera {
    // extrinsic properties
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub pitch: f64,
    pub yaw: f64,
    // intrinsic properties
    pub image_fov: f64,
    pub image_width: u32,
    pub image_height: u32,
    pub update_rate: u32,
}

impl PhysicalCamera {
    pub fn transform(&self) -> Transform {
        Transform {
            translation: Vec3::new(self.x as f32, self.y as f32, self.z as f32),
            //EulerRot::ZYX means apply yaw, pitch, roll in that order
            rotation: Quat::from_euler(
                EulerRot::ZYX,
                self.yaw as f32,
                self.pitch as f32,
                0.0 as f32,
            ),
            scale: Vec3::new(0.1, 0.1, 0.1),
            ..Default::default()
        }
    }
}

/// An axis-aligned box with a pyramid on its right face.
/// Defined by the box minimum and maximum point and the pyramid tip.
#[derive(Debug, Copy, Clone)]
pub struct DirectionalBox {
    pub min_x: f32,
    pub max_x: f32,

    pub min_y: f32,
    pub max_y: f32,

    pub min_z: f32,
    pub max_z: f32,

    pub py_tip_x: f32,
}

impl DirectionalBox {
    /// Creates a new box centered at the origin with the supplied side lengths.
    /// Adds a pyramid on the box's right face.
    pub fn new(x_length: f32, y_length: f32, z_length: f32) -> DirectionalBox {
        DirectionalBox {
            max_x: x_length / 2.0,
            min_x: -x_length / 2.0,
            max_y: y_length / 2.0,
            min_y: -y_length / 2.0,
            max_z: z_length / 2.0,
            min_z: -z_length / 2.0,
            py_tip_x: x_length,
        }
    }
}

impl Default for DirectionalBox {
    fn default() -> Self {
        DirectionalBox::new(2.0, 1.0, 1.0)
    }
}

impl From<DirectionalBox> for Mesh {
    fn from(sp: DirectionalBox) -> Self {
        let vertices = &[
            // Top
            ([sp.min_x, sp.min_y, sp.max_z], [0., 0., 1.0], [0., 0.]),
            ([sp.max_x, sp.min_y, sp.max_z], [0., 0., 1.0], [1.0, 0.]),
            ([sp.max_x, sp.max_y, sp.max_z], [0., 0., 1.0], [1.0, 1.0]),
            ([sp.min_x, sp.max_y, sp.max_z], [0., 0., 1.0], [0., 1.0]),
            // Bottom
            ([sp.min_x, sp.max_y, sp.min_z], [0., 0., -1.0], [1.0, 0.]),
            ([sp.max_x, sp.max_y, sp.min_z], [0., 0., -1.0], [0., 0.]),
            ([sp.max_x, sp.min_y, sp.min_z], [0., 0., -1.0], [0., 1.0]),
            ([sp.min_x, sp.min_y, sp.min_z], [0., 0., -1.0], [1.0, 1.0]),
            // Left
            ([sp.min_x, sp.min_y, sp.max_z], [-1.0, 0., 0.], [1.0, 0.]),
            ([sp.min_x, sp.max_y, sp.max_z], [-1.0, 0., 0.], [0., 0.]),
            ([sp.min_x, sp.max_y, sp.min_z], [-1.0, 0., 0.], [0., 1.0]),
            ([sp.min_x, sp.min_y, sp.min_z], [-1.0, 0., 0.], [1.0, 1.0]),
            // Front
            ([sp.max_x, sp.max_y, sp.min_z], [0., 1.0, 0.], [1.0, 0.]),
            ([sp.min_x, sp.max_y, sp.min_z], [0., 1.0, 0.], [0., 0.]),
            ([sp.min_x, sp.max_y, sp.max_z], [0., 1.0, 0.], [0., 1.0]),
            ([sp.max_x, sp.max_y, sp.max_z], [0., 1.0, 0.], [1.0, 1.0]),
            // Back
            ([sp.max_x, sp.min_y, sp.max_z], [0., -1.0, 0.], [0., 0.]),
            ([sp.min_x, sp.min_y, sp.max_z], [0., -1.0, 0.], [1.0, 0.]),
            ([sp.min_x, sp.min_y, sp.min_z], [0., -1.0, 0.], [1.0, 1.0]),
            ([sp.max_x, sp.min_y, sp.min_z], [0., -1.0, 0.], [0., 1.0]),
            //
            // Pyramid Top
            ([sp.max_x, sp.max_y, sp.max_z], [FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2], [1.0, 1.0]),
            ([sp.max_x, sp.min_y, sp.max_z], [FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2], [0., 1.0]),
            ([sp.py_tip_x, 0.0, 0.0], [FRAC_1_SQRT_2, 0., FRAC_1_SQRT_2], [1.0, 0.]),
            // Pyramid Bottom
            ([sp.max_x, sp.min_y, sp.min_z], [FRAC_1_SQRT_2, 0., -FRAC_1_SQRT_2], [0., 0.]),
            ([sp.max_x, sp.max_y, sp.min_z], [FRAC_1_SQRT_2, 0., -FRAC_1_SQRT_2], [1.0, 0.]),
            ([sp.py_tip_x, 0.0, 0.0], [FRAC_1_SQRT_2, 0., -FRAC_1_SQRT_2], [0., 1.0]),
            // Pyramid Front
            ([sp.max_x, sp.max_y, sp.min_z], [FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.], [1.0, 0.]),
            ([sp.max_x, sp.max_y, sp.max_z], [FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.], [1.0, 1.0]),
            ([sp.py_tip_x, 0.0, 0.0], [FRAC_1_SQRT_2, FRAC_1_SQRT_2, 0.], [0., 1.0]),
            // Pyramid Back
            ([sp.max_x, sp.min_y, sp.max_z], [FRAC_1_SQRT_2, -FRAC_1_SQRT_2, 0.], [0., 1.0]),
            ([sp.max_x, sp.min_y, sp.min_z], [FRAC_1_SQRT_2, -FRAC_1_SQRT_2, 0.], [0., 0.]),
            ([sp.py_tip_x, 0.0, 0.0], [FRAC_1_SQRT_2, -FRAC_1_SQRT_2, 0.], [1.0, 0.]),
        ];

        let mut positions = Vec::with_capacity(32);
        let mut normals = Vec::with_capacity(32);
        let mut uvs = Vec::with_capacity(32);

        for (position, normal, uv) in vertices.iter() {
            positions.push(*position);
            normals.push(*normal);
            uvs.push(*uv);
        }

        let indices = Indices::U32(vec![
            0, 1, 2, 2, 3, 0, // top
            4, 5, 6, 6, 7, 4, // bottom
            8, 9, 10, 10, 11, 8, // left
            12, 13, 14, 14, 15, 12, // front
            16, 17, 18, 18, 19, 16, // back
            20, 21, 22, // pyramid top
            23, 24, 25, // pyramid bottom
            26, 27, 28, // pyramid front
            29, 30, 31, // pyramid back
        ]);

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(indices));
        mesh
    }
}