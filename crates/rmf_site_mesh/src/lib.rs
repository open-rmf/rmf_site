use std::{
    cmp,
    ops::{Add, Mul, Neg},
};

use bevy_asset::{RenderAssetUsages, prelude::*};
use bevy_color::{Color, LinearRgba};
use bevy_derive::Deref;
use bevy_math::Affine3A;
use bevy_mod_outline::ATTRIBUTE_OUTLINE_NORMAL;
use bevy_reflect::prelude::*;
use bevy_render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues};

pub mod faces;
pub use faces::*;

pub mod primitives;
pub use primitives::*;

pub mod traits;
pub use traits::*;
pub enum Angle {
    Deg(Degrees),
    Rad(Radians),
}

#[derive(Deref, Clone, Copy, PartialEq, PartialOrd)]
pub struct Radians(pub f32);

impl From<Radians> for Angle {
    fn from(value: Radians) -> Self {
        Angle::Rad(value)
    }
}

impl From<Angle> for Radians {
    fn from(value: Angle) -> Self {
        match value {
            Angle::Deg(degrees) => Radians(degrees.to_radians()),
            Angle::Rad(radians) => radians,
        }
    }
}

impl PartialEq<f32> for Radians {
    fn eq(&self, other: &f32) -> bool {
        &self.0 == other
    }
}

impl PartialOrd<f32> for Radians {
    fn partial_cmp(&self, other: &f32) -> Option<cmp::Ordering> {
        self.0.partial_cmp(other)
    }
}

impl Neg for Radians {
    type Output = Self;

    fn neg(self) -> Self::Output {
        Self(-self.0)
    }
}

impl Add for Radians {
    type Output = Self;

    fn add(self, rhs: Self) -> Self::Output {
        Self(self.0 + rhs.0)
    }
}

#[derive(Deref, PartialEq, PartialOrd)]
pub struct Degrees(pub f32);

impl From<Degrees> for Angle {
    fn from(value: Degrees) -> Self {
        Angle::Deg(value)
    }
}

impl Mul for Degrees {
    type Output = Self;

    fn mul(self, rhs: Self) -> Self::Output {
        Self(self.0 * rhs.0)
    }
}

impl Mul<f32> for Degrees {
    type Output = Self;

    fn mul(self, rhs: f32) -> Self::Output {
        Self(self.0 * rhs)
    }
}

impl From<Degrees> for Radians {
    fn from(value: Degrees) -> Self {
        Self(value.to_radians())
    }
}

impl From<Radians> for Degrees {
    fn from(value: Radians) -> Self {
        Self(value.to_degrees())
    }
}

impl From<Angle> for Degrees {
    fn from(value: Angle) -> Self {
        match value {
            Angle::Deg(degrees) => degrees,
            Angle::Rad(radians) => Degrees(radians.to_degrees()),
        }
    }
}

#[derive(Default, Debug, Clone)]
pub struct MeshBuffer {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    outline: Vec<u32>,
    uv: Option<Vec<[f32; 2]>>,
    copy_outline_normals: bool,
}

impl From<MeshBuffer> for Mesh {
    fn from(buffer: MeshBuffer) -> Self {
        let mut mesh = Mesh::new(
            PrimitiveTopology::TriangleList,
            RenderAssetUsages::default(),
        );
        mesh.insert_indices(Indices::U32(buffer.indices));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, buffer.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, buffer.normals);
        if let Some(uv) = buffer.uv {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        }
        mesh
    }
}

impl MeshBuffer {
    pub fn new(positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, indices: Vec<u32>) -> Self {
        if positions.len() != normals.len() {
            panic!(
                "Inconsistent positions {} vs normals {}",
                positions.len(),
                normals.len(),
            );
        }

        Self {
            positions,
            normals,
            indices,
            outline: Vec::new(),
            uv: None,
            copy_outline_normals: false,
        }
    }

    pub fn empty() -> Self {
        Self::default()
    }

    pub fn copy_outline_normals(mut self) -> Self {
        self.copy_outline_normals = true;
        self
    }

    pub fn with_outline(mut self, outline: Vec<u32>) -> Self {
        self.outline = outline;
        self
    }

    pub fn with_uv(mut self, uv: Vec<[f32; 2]>) -> Self {
        if uv.len() != self.positions.len() {
            panic!(
                "Inconsistent positions {} vs uv {}",
                self.positions.len(),
                uv.len()
            );
        }
        self.uv = Some(uv);
        self
    }

    pub fn transform_by(mut self, tf: Affine3A) -> Self {
        for p in &mut self.positions {
            *p = tf.transform_point3((*p).into()).into();
        }

        for n in &mut self.normals {
            *n = tf.transform_vector3((*n).into()).into();
        }

        self
    }

    pub fn merge_with(mut self, other: Self) -> Self {
        let offset = self.positions.len();
        self.indices
            .extend(other.indices.into_iter().map(|i| i + offset as u32));
        self.outline
            .extend(other.outline.into_iter().map(|i| i + offset as u32));
        self.positions.extend(other.positions.into_iter());
        self.normals.extend(other.normals.into_iter());

        // Only keep the UV property if both meshes contain it. Otherwise drop it.
        if let (Some(mut uv), Some(other_uv)) = (self.uv, other.uv) {
            uv.extend(other_uv);
            self.uv = Some(uv);
        } else {
            self.uv = None;
        }

        self
    }

    pub fn merge_into(self, mesh: &mut Mesh) {
        let offset = mesh.attribute(Mesh::ATTRIBUTE_POSITION).map(|a| a.len());
        if let Some(offset) = offset {
            match mesh.primitive_topology() {
                PrimitiveTopology::TriangleList => {
                    if let Some(Indices::U32(indices)) = mesh.indices_mut() {
                        indices.extend(self.indices.into_iter().map(|i| i + offset as u32));
                    } else {
                        mesh.insert_indices(Indices::U32(
                            self.indices
                                .into_iter()
                                .map(|i| i + offset as u32)
                                .collect(),
                        ));
                    }
                }
                PrimitiveTopology::LineList => {
                    if let Some(Indices::U32(indices)) = mesh.indices_mut() {
                        indices.extend(self.outline.into_iter().map(|i| i + offset as u32));
                    } else {
                        mesh.insert_indices(Indices::U32(
                            self.outline
                                .into_iter()
                                .map(|i| i + offset as u32)
                                .collect(),
                        ));
                    }
                }
                other => {
                    panic!(
                        "Unsupported primitive topology while merging mesh: {:?}",
                        other
                    );
                }
            }

            if self.copy_outline_normals {
                if let Some(VertexAttributeValues::Float32x3(current_outline_normals)) =
                    mesh.attribute_mut(ATTRIBUTE_OUTLINE_NORMAL)
                {
                    current_outline_normals.extend(self.normals.clone().into_iter());
                } else {
                    let mut normals =
                        if let Some(VertexAttributeValues::Float32x3(current_normals)) =
                            mesh.attribute(Mesh::ATTRIBUTE_NORMAL)
                        {
                            current_normals.clone()
                        } else {
                            Vec::new()
                        };

                    normals.extend(self.normals.clone().into_iter());
                    mesh.insert_attribute(ATTRIBUTE_OUTLINE_NORMAL, normals);
                }
            }

            if let Some(VertexAttributeValues::Float32x3(current_positions)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION)
            {
                current_positions.extend(self.positions.into_iter());

                if let Some(VertexAttributeValues::Float32x3(current_normals)) =
                    mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL)
                {
                    current_normals.extend(self.normals.into_iter());
                } else {
                    panic!("Mesh is missing normals attribute when it has positions attribute!");
                }
            } else {
                panic!("Unsupported position type while merging mesh");
            }

            if let Some(VertexAttributeValues::Float32x2(current_uvs)) =
                mesh.attribute_mut(Mesh::ATTRIBUTE_UV_0)
            {
                if let Some(new_uvs) = self.uv {
                    current_uvs.extend(new_uvs);
                } else {
                    panic!("Mesh needs UV values but the buffer does not have any!");
                }
            }
        } else {
            // The mesh currently has no positions in it (and should therefore have no normals or indices either)
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
            if let Some(uv) = self.uv {
                mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
            }

            match mesh.primitive_topology() {
                PrimitiveTopology::TriangleList => {
                    mesh.insert_indices(Indices::U32(self.indices));
                }
                PrimitiveTopology::LineList => {
                    mesh.insert_indices(Indices::U32(self.outline));
                }
                other => {
                    panic!(
                        "Unsupported primitive topology while merging mesh: {:?}",
                        other
                    );
                }
            }
        }
    }

    pub fn into_outline(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::LineList, RenderAssetUsages::default());
        mesh.insert_indices(Indices::U32(self.outline));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh
    }

    pub fn into_mesh_and_outline(self) -> (Mesh, Mesh) {
        let clone = self.clone();
        (clone.into(), self.into_outline())
    }
}

// TODO(@xiyuoh) Temp only: ported from bevy_polyline, use GizmoConfig to instead
#[derive(Asset, Debug, PartialEq, Clone, Copy, TypePath)]
pub struct PolylineMaterial {
    /// Width of the line.
    ///
    /// Corresponds to screen pixels when line is positioned nearest the
    /// camera.
    pub width: f32,
    pub color: LinearRgba,
    /// How closer to the camera than real geometry the line should be.
    ///
    /// Value between -1 and 1 (inclusive).
    /// * 0 means that there is no change to the line position when rendering
    /// * 1 means it is furthest away from camera as possible
    /// * -1 means that it will always render in front of other things.
    ///
    /// This is typically useful if you are drawing wireframes on top of polygons
    /// and your wireframe is z-fighting (flickering on/off) with your main model.
    /// You would set this value to a negative number close to 0.0.
    pub depth_bias: f32,
    /// Whether to reduce line width with perspective.
    ///
    /// When `perspective` is `true`, `width` corresponds to screen pixels at
    /// the near plane and becomes progressively smaller further away. This is done
    /// by dividing `width` by the w component of the homogeneous coordinate.
    ///
    /// If the width where to be lower than 1, the color of the line is faded. This
    /// prevents flickering.
    ///
    /// Note that `depth_bias` **does not** interact with this in any way.
    pub perspective: bool,
}

impl Default for PolylineMaterial {
    fn default() -> Self {
        Self {
            width: 10.0,
            color: Color::WHITE.to_linear(),
            depth_bias: 0.0,
            perspective: false,
        }
    }
}
