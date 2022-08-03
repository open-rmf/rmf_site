use bevy::prelude::*;
use bevy::render::mesh::Indices;
use bevy::render::mesh::PrimitiveTopology;
use serde::{Deserialize, Serialize};

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

/// An axis-aligned square-based pyramid defined by its minimum and maximum point.
#[derive(Debug, Copy, Clone)]
pub struct Pyramid {
    pub ht: f32,

    pub min_y: f32,
    pub max_y: f32,

    pub min_z: f32,
    pub max_z: f32,
}

impl Pyramid {
    /// Creates a new square-based pyramid with base centered at the origin with the supplied lengths.
    pub fn new(base_length: f32, height: f32) -> Pyramid {
        Pyramid {
            ht: height,
            max_y: base_length / 2.0,
            min_y: -base_length / 2.0,
            max_z: base_length / 2.0,
            min_z: -base_length / 2.0,
        }
    }
}

impl Default for Pyramid {
    fn default() -> Self {
        Pyramid::new(1.0, 1.0)
    }
}

impl From<Pyramid> for Mesh {
    fn from(sp: Pyramid) -> Self {
        let vertices = &[
            // Base (facing left)
            ([0., sp.min_y, sp.max_z], [-1.0, 0., 0.], [1.0, 0.]),
            ([0., sp.max_y, sp.max_z], [-1.0, 0., 0.], [0., 0.]),
            ([0., sp.max_y, sp.min_z], [-1.0, 0., 0.], [0., 1.0]),
            ([0., sp.min_y, sp.min_z], [-1.0, 0., 0.], [1.0, 1.0]),
            // Pyramid Top
            ([0., sp.max_y, sp.max_z], [sp.max_z, 0., sp.ht], [1.0, 1.0]),
            ([0., sp.min_y, sp.max_z], [sp.max_z, 0., sp.ht], [0., 1.0]),
            ([sp.ht, 0., 0.], [sp.max_z, 0., sp.ht], [1.0, 0.]),
            // Pyramid Bottom
            ([0., sp.min_y, sp.min_z], [sp.max_z, 0., -sp.ht], [0., 0.]),
            ([0., sp.max_y, sp.min_z], [sp.max_z, 0., -sp.ht], [1.0, 0.]),
            ([sp.ht, 0., 0.], [sp.max_z, 0., -sp.ht], [0., 1.0]),
            // Pyramid Front
            ([0., sp.max_y, sp.min_z], [sp.max_y, sp.ht, 0.], [1.0, 0.]),
            ([0., sp.max_y, sp.max_z], [sp.max_y, sp.ht, 0.], [1.0, 1.0]),
            ([sp.ht, 0., 0.], [sp.max_y, sp.ht, 0.], [0., 1.0]),
            // Pyramid Back
            ([0., sp.min_y, sp.max_z], [sp.max_y, -sp.ht, 0.], [0., 1.0]),
            ([0., sp.min_y, sp.min_z], [sp.max_y, -sp.ht, 0.], [0., 0.]),
            ([sp.ht, 0., 0.], [sp.max_y, -sp.ht, 0.], [1.0, 0.]),
        ];

        let positions: Vec<_> = vertices.iter().map(|(p, _, _)| *p).collect();
        let normals: Vec<_> = vertices.iter().map(|(_, n, _)| *n).collect();
        let uvs: Vec<_> = vertices.iter().map(|(_, _, uv)| *uv).collect();

        let indices = Indices::U32(vec![
            0, 1, 2, 2, 3, 0, // base (facing left)
            4, 5, 6, // pyramid top
            7, 8, 9, // pyramid bottom
            10, 11, 12, // pyramid front
            13, 14, 15, // pyramid back
        ]);

        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
        mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uvs);
        mesh.set_indices(Some(indices));
        mesh
    }
}
