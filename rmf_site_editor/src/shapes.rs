/*
 * Copyright (C) 2022 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

use bevy::math::Affine3A;
use bevy::{
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
        primitives::Aabb,
    },
};
use rmf_site_format::Angle;

#[derive(Default, Debug, Clone)]
pub(crate) struct MeshBuffer {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
    outline: Vec<u32>,
    uv: Option<Vec<[f32; 2]>>,
}

impl MeshBuffer {
    pub(crate) fn new(positions: Vec<[f32; 3]>, normals: Vec<[f32; 3]>, indices: Vec<u32>) -> Self {
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
        }
    }

    pub(crate) fn empty() -> Self {
        Self::default()
    }

    pub(crate) fn with_outline(mut self, outline: Vec<u32>) -> Self {
        self.outline = outline;
        self
    }

    pub(crate) fn with_uv(mut self, uv: Vec<[f32; 2]>) -> Self {
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

    pub(crate) fn transform_by(mut self, tf: Affine3A) -> Self {
        for p in &mut self.positions {
            *p = tf.transform_point3((*p).into()).into();
        }

        for n in &mut self.normals {
            *n = tf.transform_vector3((*n).into()).into();
        }

        self
    }

    pub(crate) fn merge_with(mut self, other: Self) -> Self {
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

    pub(crate) fn merge_into(self, mesh: &mut Mesh) {
        let offset = mesh.attribute(Mesh::ATTRIBUTE_POSITION).map(|a| a.len());
        if let Some(offset) = offset {
            match mesh.primitive_topology() {
                PrimitiveTopology::TriangleList => {
                    if let Some(Indices::U32(indices)) = mesh.indices_mut() {
                        indices.extend(self.indices.into_iter().map(|i| i + offset as u32));
                    } else {
                        mesh.set_indices(Some(Indices::U32(
                            self.indices
                                .into_iter()
                                .map(|i| i + offset as u32)
                                .collect(),
                        )));
                    }
                }
                PrimitiveTopology::LineList => {
                    if let Some(Indices::U32(indices)) = mesh.indices_mut() {
                        indices.extend(self.outline.into_iter().map(|i| i + offset as u32));
                    } else {
                        mesh.set_indices(Some(Indices::U32(
                            self.outline
                                .into_iter()
                                .map(|i| i + offset as u32)
                                .collect(),
                        )));
                    }
                }
                other => {
                    panic!(
                        "Unsupported primitive topology while merging mesh: {:?}",
                        other
                    );
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
                    mesh.set_indices(Some(Indices::U32(self.indices)));
                }
                PrimitiveTopology::LineList => {
                    mesh.set_indices(Some(Indices::U32(self.outline)));
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

    pub(crate) fn into_outline(self) -> Mesh {
        let mut mesh = Mesh::new(PrimitiveTopology::LineList);
        mesh.set_indices(Some(Indices::U32(self.outline)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
        mesh
    }

    pub(crate) fn into_mesh_and_outline(self) -> (Mesh, Mesh) {
        let clone = self.clone();
        (clone.into(), self.into_outline())
    }
}

impl From<MeshBuffer> for Mesh {
    fn from(buffer: MeshBuffer) -> Self {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(Indices::U32(buffer.indices)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, buffer.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, buffer.normals);
        if let Some(uv) = buffer.uv {
            mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);
        }
        mesh
    }
}

#[derive(Clone, Copy, Debug)]
pub struct Circle {
    pub radius: f32,
    pub height: f32,
}

impl Circle {
    fn flip_height(mut self) -> Self {
        self.height = -self.height;
        self
    }
}

impl From<(f32, f32)> for Circle {
    fn from((radius, height): (f32, f32)) -> Self {
        Self { radius, height }
    }
}

pub(crate) fn make_circles(
    circles: impl IntoIterator<Item = Circle>,
    resolution: u32,
    gap: f32,
) -> impl Iterator<Item = [f32; 3]> {
    return [0..resolution]
        .into_iter()
        .cycle()
        .zip(circles.into_iter())
        .flat_map(move |(range, circle)| {
            range.into_iter().map(move |i| {
                let theta = (i as f32) / (resolution as f32 - 1.) * (std::f32::consts::TAU - gap);
                let r = circle.radius;
                let h = circle.height;
                [r * theta.cos(), r * theta.sin(), h]
            })
        });
}

pub(crate) fn make_boxy_wrap(circles: [Circle; 2], segments: u32) -> MeshBuffer {
    let (bottom_circle, top_circle) = if circles[0].height < circles[1].height {
        (circles[0], circles[1])
    } else {
        (circles[1], circles[0])
    };

    let positions: Vec<[f32; 3]> = make_circles(
        [bottom_circle, bottom_circle, top_circle, top_circle],
        segments + 1,
        0.,
    )
    .collect();

    let indices = [[
        0,
        3 * segments + 4,
        2 * segments + 2,
        0,
        segments + 2,
        3 * segments + 4,
    ]]
    .into_iter()
    .cycle()
    .enumerate()
    .flat_map(|(i, values)| values.into_iter().map(move |s| s + i as u32))
    .take(6 * segments as usize)
    .collect();

    let mut normals = Vec::new();
    normals.resize(positions.len(), [0., 0., 0.]);
    for i in 0..segments {
        let v0 = (i + 0) as usize;
        let v1 = (i + 3 * segments + 4) as usize;
        let v2 = (i + 2 * segments + 2) as usize;
        let v3 = (i + segments + 2) as usize;
        let p0: Vec3 = positions[v0].into();
        let p1: Vec3 = positions[v1].into();
        let p2: Vec3 = positions[v2].into();
        let n = (p1 - p0).cross(p2 - p0).normalize();
        [v0, v1, v2, v3].into_iter().for_each(|v| {
            normals[v] = n.into();
        });
    }

    return MeshBuffer::new(positions, normals, indices);
}

pub(crate) fn make_smooth_wrap(circles: [Circle; 2], resolution: u32) -> MeshBuffer {
    let (bottom_circle, top_circle) = if circles[0].height < circles[1].height {
        (circles[0], circles[1])
    } else {
        (circles[1], circles[0])
    };

    let positions: Vec<[f32; 3]> =
        make_circles([bottom_circle, top_circle], resolution, 0.).collect();

    let top_start = resolution;
    let indices = [[0, 1, top_start + 1, 0, top_start + 1, top_start]]
        .into_iter()
        .cycle()
        .enumerate()
        .flat_map(|(i, values)| values.into_iter().map(move |s| s + i as u32))
        .take(6 * (resolution - 1) as usize)
        .collect();

    let mut normals = Vec::new();
    normals.resize(positions.len(), [0., 0., 1.]);
    for i in 0..resolution {
        let theta = (i as f32) / (resolution as f32 - 1.) * 2. * std::f32::consts::PI;
        let dr = top_circle.radius - bottom_circle.radius;
        let dh = top_circle.height - bottom_circle.height;
        let phi = dr.atan2(dh);
        let r_y = Affine3A::from_rotation_y(phi);
        let r_z = Affine3A::from_rotation_z(theta);
        let n = (r_z * r_y).transform_vector3([1., 0., 0.].into());
        normals[i as usize] = n.into();
        normals[(i + top_start) as usize] = n.into();
    }

    return MeshBuffer::new(positions, normals, indices);
}

pub(crate) fn make_pyramid(circle: Circle, peak: [f32; 3], segments: u32) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = make_circles([circle, circle], segments + 1, 0.)
        .chain([peak].into_iter().cycle().take(segments as usize))
        .collect();

    let peak_start = 2 * segments + 2;
    let complement_start = segments + 2;
    let indices = [[0, complement_start, peak_start]]
        .into_iter()
        .cycle()
        .enumerate()
        .flat_map(|(i, values)| values.into_iter().map(move |s| s + i as u32))
        .take(3 * segments as usize)
        .collect();

    let mut normals = Vec::new();
    normals.resize(positions.len(), [0., 0., 0.]);
    for i in 0..segments {
        let v0 = (i + 0) as usize;
        let v1 = (i + complement_start) as usize;
        let vp = (i + peak_start) as usize;
        let p0: Vec3 = positions[v0].into();
        let p1: Vec3 = positions[v1].into();
        let p2: Vec3 = positions[vp].into();
        let n = if peak[2] < circle.height {
            (p2 - p0).cross(p1 - p0)
        } else {
            (p1 - p0).cross(p2 - p0)
        }
        .normalize();

        [v0, v1, vp].into_iter().for_each(|v| {
            normals[v] = n.into();
        });
    }

    return MeshBuffer::new(positions, normals, indices);
}

pub(crate) fn make_cone(circle: Circle, peak: [f32; 3], resolution: u32) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = make_circles([circle], resolution + 1, 0.)
        .take(resolution as usize) // skip the last vertex which would close the circle
        .chain([peak].into_iter().cycle().take(resolution as usize))
        .collect();

    let peak_start = resolution;
    let indices: Vec<u32> = [[0, 1, peak_start]]
        .into_iter()
        .cycle()
        .enumerate()
        .flat_map(|(i, values)| values.into_iter().map(move |s| s + i as u32))
        .take(3 * (resolution as usize - 1))
        .chain([peak_start - 1, 0, (positions.len() - 1) as u32])
        .collect();

    let mut normals = Vec::<[f32; 3]>::new();
    let base_p = Vec3::new(peak[0], peak[1], circle.height);
    normals.resize(positions.len(), [0., 0., 1.]);
    for i in 0..resolution {
        // Normals around the ring
        let calculate_normal = |theta: f32| -> [f32; 3] {
            let p = circle.radius * Vec3::new(theta.cos(), theta.sin(), circle.height);
            let r = (p - base_p).length();
            let h = peak[2] - circle.height;
            let phi = r.atan2(h);
            let r_y = Affine3A::from_rotation_y(-phi);
            let r_z = Affine3A::from_rotation_z(theta);
            (r_z * r_y).transform_vector3(Vec3::new(1., 0., 0.)).into()
        };

        let theta = (i as f32) / (resolution as f32) * 2.0 * std::f32::consts::PI;
        normals[i as usize] = calculate_normal(theta);

        let mid_theta = (i as f32 + 0.5) / (resolution as f32) * 2.0 * std::f32::consts::PI;
        normals[(i + peak_start) as usize] = calculate_normal(mid_theta);
    }

    return MeshBuffer::new(positions, normals, indices);
}

pub(crate) fn make_box(x_extent: f32, y_extent: f32, z_extent: f32) -> MeshBuffer {
    let (min_x, max_x) = (-x_extent, x_extent);
    let (min_y, max_y) = (-y_extent, y_extent);
    let (min_z, max_z) = (-z_extent, z_extent);
    let vertices = &[
        // Top
        ([min_x, min_y, max_z], [0., 0., 1.]),
        ([max_x, min_y, max_z], [0., 0., 1.]),
        ([max_x, max_y, max_z], [0., 0., 1.]),
        ([min_x, max_y, max_z], [0., 0., 1.]),
        // Bottom
        ([min_x, max_y, min_z], [0., 0., -1.]),
        ([max_x, max_y, min_z], [0., 0., -1.]),
        ([max_x, min_y, min_z], [0., 0., -1.]),
        ([min_x, min_y, min_z], [0., 0., -1.]),
        // Right
        ([max_x, min_y, min_z], [1., 0., 0.]),
        ([max_x, max_y, min_z], [1., 0., 0.]),
        ([max_x, max_y, max_z], [1., 0., 0.]),
        ([max_x, min_y, max_z], [1., 0., 0.]),
        // Left
        ([min_x, min_y, max_z], [-1., 0., 0.]),
        ([min_x, max_y, max_z], [-1., 0., 0.]),
        ([min_x, max_y, min_z], [-1., 0., 0.]),
        ([min_x, min_y, min_z], [-1., 0., 0.]),
        // Front
        ([max_x, max_y, min_z], [0., 1., 0.]),
        ([min_x, max_y, min_z], [0., 1., 0.]),
        ([min_x, max_y, max_z], [0., 1., 0.]),
        ([max_x, max_y, max_z], [0., 1., 0.]),
        // Back
        ([max_x, min_y, max_z], [0., -1., 0.]),
        ([min_x, min_y, max_z], [0., -1., 0.]),
        ([min_x, min_y, min_z], [0., -1., 0.]),
        ([max_x, min_y, min_z], [0., -1., 0.]),
    ];

    let positions: Vec<_> = vertices.iter().map(|(p, _)| *p).collect();
    let normals: Vec<_> = vertices.iter().map(|(_, n)| *n).collect();
    let indices = vec![
        0, 1, 2, 2, 3, 0, // Top
        4, 5, 6, 6, 7, 4, // Bottom
        8, 9, 10, 10, 11, 8, // Right
        12, 13, 14, 14, 15, 12, // Left
        16, 17, 18, 18, 19, 16, // Front
        20, 21, 22, 22, 23, 20, // Back
    ];

    MeshBuffer::new(positions, normals, indices)
}

pub(crate) fn make_wall_mesh(
    p_start: Vec3,
    p_end: Vec3,
    thickness: f32,
    height: f32,
) -> MeshBuffer {
    let dp = p_end - p_start;
    let length = dp.length();
    let yaw = dp.y.atan2(dp.x);
    let center = (p_start + p_end) / 2.0;

    // The default UV coordinates made by bevy do not work well for walls,
    // so we customize them here
    let uv = vec![
        // Top
        [0., 0.], // 0
        [0., 0.], // 1
        [0., 0.], // 2
        [0., 0.], // 3
        // Bottom
        [0., 1.], // 4
        [0., 1.], // 5
        [0., 1.], // 6
        [0., 1.], // 7
        // Right
        [length, 1.], // 8
        [0., 1.],     // 9
        [0., 0.],     // 10
        [length, 0.], // 11
        // Left
        [0., 0.],     // 12
        [length, 0.], // 13
        [length, 1.], // 14
        [0., 1.],     // 15
        // Front
        [0., 1.],     // 16
        [length, 1.], // 17
        [length, 0.], // 18
        [0., 0.],     // 19
        // Back
        [length, 0.], // 20
        [0., 0.],     // 21
        [0., 1.],     // 22
        [length, 1.], // 23
    ];
    make_box(length / 2.0, thickness / 2.0, height / 2.0)
        .with_uv(uv)
        .transform_by(
            Affine3A::from_translation(Vec3::new(center.x, center.y, height / 2.0))
                * Affine3A::from_rotation_z(yaw),
        )
}

pub(crate) fn make_top_circle(circle: Circle, resolution: u32) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = make_circles([circle], resolution, 0.)
        .take(resolution as usize) // skip the vertex which would close the circle
        .chain([[0., 0., circle.height]].into_iter())
        .collect();

    let peak = positions.len() as u32 - 1;
    let indices: Vec<u32> = (0..=peak - 2)
        .into_iter()
        .flat_map(|i| [i, i + 1, peak].into_iter())
        .chain([peak - 1, 0, peak])
        .collect();

    let normals: Vec<[f32; 3]> = [[0., 0., 1.]]
        .into_iter()
        .cycle()
        .take(positions.len())
        .collect();

    return MeshBuffer::new(positions, normals, indices);
}

pub(crate) fn make_bottom_circle(circle: Circle, resolution: u32) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = make_circles([circle], resolution, 0.)
        .take(resolution as usize) // skip the vertex which would close the circle
        .chain([[0., 0., circle.height]].into_iter())
        .collect();

    let peak = positions.len() as u32 - 1;
    let indices: Vec<u32> = (0..=peak - 2)
        .into_iter()
        .flat_map(|i| [i, peak, i + 1].into_iter())
        .chain([peak - 1, peak, 0])
        .collect();

    let normals: Vec<[f32; 3]> = [[0., 0., -1.]]
        .into_iter()
        .cycle()
        .take(positions.len())
        .collect();

    return MeshBuffer::new(positions, normals, indices);
}

pub(crate) fn make_flat_disk(circle: Circle, resolution: u32) -> MeshBuffer {
    make_top_circle(circle, resolution).merge_with(make_bottom_circle(circle, resolution))
}

pub(crate) fn make_dagger_mesh() -> Mesh {
    let lower_ring = Circle {
        radius: 0.01,
        height: 0.1,
    };
    let upper_ring = Circle {
        radius: 0.02,
        height: 0.4,
    };
    let top_height = 0.42;
    let segments = 4u32;

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    make_boxy_wrap([lower_ring, upper_ring], segments).merge_into(&mut mesh);
    make_pyramid(upper_ring, [0., 0., top_height], segments).merge_into(&mut mesh);
    make_pyramid(lower_ring.flip_height(), [0., 0., 0.], segments)
        .transform_by(Affine3A::from_quat(Quat::from_rotation_y(
            180_f32.to_radians(),
        )))
        .merge_into(&mut mesh);
    return mesh;
}

pub(crate) fn make_cylinder(height: f32, radius: f32) -> MeshBuffer {
    let top_circle = Circle { height, radius };
    let mid_circle = Circle {
        height: 0.0,
        radius,
    };
    let bottom_circle = Circle {
        height: -height,
        radius,
    };
    let resolution = 32;
    make_smooth_wrap([top_circle, bottom_circle], resolution)
        .merge_with(
            make_bottom_circle(mid_circle, resolution)
                .transform_by(Affine3A::from_translation([0.0, 0., -height].into())),
        )
        .merge_with(make_bottom_circle(mid_circle, resolution).transform_by(
            Affine3A::from_translation([0., 0., height].into())
                * Affine3A::from_rotation_x(180_f32.to_radians()),
        ))
}

pub(crate) fn make_cylinder_arrow_mesh() -> Mesh {
    let tip = [0., 0., 1.];
    let l_head = 0.2;
    let r_head = 0.15;
    let r_base = 0.1;
    let head_base = Circle {
        radius: r_head,
        height: 1. - l_head,
    };
    let cylinder_top = Circle {
        radius: r_base,
        height: 1. - l_head,
    };
    let cylinder_bottom = Circle {
        radius: r_base,
        height: 0.0,
    };
    let resolution = 32u32;

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    make_cone(head_base, tip, resolution).merge_into(&mut mesh);
    make_smooth_wrap([cylinder_top, cylinder_bottom], resolution).merge_into(&mut mesh);
    make_smooth_wrap([head_base, cylinder_top], resolution).merge_into(&mut mesh);
    make_bottom_circle(cylinder_bottom, resolution).merge_into(&mut mesh);
    return mesh;
}

pub(crate) fn flat_arrow_mesh(
    handle_length: f32,
    handle_width: f32,
    tip_length: f32,
    tip_width: f32,
) -> MeshBuffer {
    let half_handle_width = handle_width / 2.0;
    let half_tip_width = tip_width / 2.0;
    let positions: Vec<[f32; 3]> = vec![
        [0.0, half_handle_width, 0.0],            // 0
        [0.0, -half_handle_width, 0.0],           // 1
        [handle_length, -half_handle_width, 0.0], // 2
        [handle_length, half_handle_width, 0.0],  // 3
        [handle_length, half_tip_width, 0.0],     // 4
        [handle_length, -half_tip_width, 0.0],    // 5
        [handle_length + tip_length, 0.0, 0.0],   // 6
    ];

    let normals: Vec<[f32; 3]> = {
        let mut normals = Vec::new();
        normals.resize(positions.len(), [0.0, 0.0, 1.0]);
        normals
    };

    let indices: Vec<u32> = vec![0, 1, 3, 1, 2, 3, 4, 5, 6];

    let outline: Vec<u32> = vec![0, 1, 1, 2, 2, 5, 5, 6, 6, 4, 4, 3, 3, 0];

    MeshBuffer::new(positions, normals, indices).with_outline(outline)
}

pub(crate) fn flat_arrow_mesh_between(
    start: Vec3,
    stop: Vec3,
    handle_width: f32,
    tip_length: f32,
    tip_width: f32,
) -> MeshBuffer {
    let total_length = (stop - start).length();
    let tip_length = total_length.min(tip_length);
    let handle_length = total_length - tip_length;
    let dp = stop - start;
    let yaw = dp.y.atan2(dp.x);

    flat_arrow_mesh(handle_length, handle_width, tip_length, tip_width).transform_by(
        Affine3A::from_scale_rotation_translation(
            Vec3::new(1.0, 1.0, 1.0),
            Quat::from_rotation_z(yaw),
            start,
        ),
    )
}

pub(crate) fn flat_arc(
    pivot: Vec3,
    outer_radius: f32,
    inner_thickness: f32,
    initial_angle: Angle,
    sweep: Angle,
    vertices_per_degree: f32,
) -> MeshBuffer {
    let (initial_angle, sweep) = if sweep.radians() < 0.0 {
        (
            Angle::Rad(initial_angle.radians() + sweep.radians()),
            Angle::Rad(-sweep.radians()),
        )
    } else {
        (initial_angle, sweep)
    };

    let resolution = (sweep.degrees() * vertices_per_degree) as u32;
    let positions: Vec<[f32; 3]> = make_circles(
        [
            (outer_radius - inner_thickness, 0.).into(),
            (outer_radius, 0.).into(),
        ],
        resolution,
        std::f32::consts::TAU - sweep.radians(),
    )
    .collect();

    let normals: Vec<[f32; 3]> = {
        let mut normals = Vec::new();
        normals.resize(positions.len(), [0.0, 0.0, 1.0]);
        normals
    };

    let indices: Vec<u32> = if resolution >= 1 {
        [[0, resolution, resolution + 1, 0, resolution + 1, 1]]
            .into_iter()
            .cycle()
            .enumerate()
            .flat_map(|(segment, values)| values.map(|s| segment as u32 + s))
            .take(6 * (resolution as usize - 1))
            .collect()
    } else {
        Vec::new()
    };

    let outline: Vec<u32> = if resolution >= 1 {
        [[0, 1, resolution, resolution + 1]]
            .into_iter()
            .cycle()
            .enumerate()
            .flat_map(|(segment, values)| values.map(|s| segment as u32 + s))
            .take(4 * (resolution as usize - 1))
            .collect()
    } else {
        Vec::new()
    };

    MeshBuffer::new(positions, normals, indices)
        .with_outline(outline)
        .transform_by(Affine3A::from_rotation_translation(
            Quat::from_rotation_z(initial_angle.radians()),
            pivot,
        ))
}

pub(crate) fn line_stroke_mesh(start: Vec3, end: Vec3, thickness: f32) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = vec![
        [-0.5, -0.5, 0.], // 0
        [0.5, -0.5, 0.],  // 1
        [0.5, 0.5, 0.],   // 2
        [-0.5, 0.5, 0.],  // 3
    ];

    let normals: Vec<[f32; 3]> = {
        let mut normals = Vec::new();
        normals.resize(positions.len(), [0.0, 0.0, 1.0]);
        normals
    };

    let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];
    let outline: Vec<u32> = vec![0, 1, 1, 2, 2, 3, 3, 0];

    let center = (start + end) / 2.0;
    let dp = end - start;
    let yaw = dp.y.atan2(dp.x);

    MeshBuffer::new(positions, normals, indices)
        .with_outline(outline)
        .transform_by(Affine3A::from_scale_rotation_translation(
            Vec3::new(dp.length(), thickness, 1.),
            Quat::from_rotation_z(yaw),
            center,
        ))
}

pub(crate) fn line_stroke_away_from(
    start: Vec3,
    direction: Angle,
    length: f32,
    thickness: f32,
) -> MeshBuffer {
    let end = start
        + Affine3A::from_rotation_z(direction.radians())
            .transform_vector3(Vec3::new(length, 0.0, 0.0));

    line_stroke_mesh(start, end, thickness)
}

pub(crate) fn make_physical_camera_mesh() -> Mesh {
    let scale = 0.1;
    let lens_hood_protrusion = 0.8;

    // Main body
    let mut mesh: Mesh = shape::Box::new(scale, scale, scale).into();
    mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0);

    // Outside of the lens hood
    make_pyramid(
        Circle {
            radius: scale,
            height: 0.,
        },
        [0., 0., scale],
        4,
    )
    .transform_by(
        Affine3A::from_translation([lens_hood_protrusion * scale, 0., 0.].into())
            * Affine3A::from_rotation_y(-90_f32.to_radians())
            * Affine3A::from_rotation_z(45_f32.to_radians()),
    )
    .merge_into(&mut mesh);

    // Inside of the lens hood
    make_pyramid(
        Circle {
            radius: scale,
            height: scale,
        },
        [0., 0., 0.],
        4,
    )
    .transform_by(
        Affine3A::from_translation([-(1.0 - lens_hood_protrusion) * scale, 0., 0.].into())
            * Affine3A::from_rotation_y(90_f32.to_radians())
            * Affine3A::from_rotation_z(45_f32.to_radians()),
    )
    .merge_into(&mut mesh);

    mesh
}

pub(crate) fn make_diamond(tip: f32, width: f32) -> MeshBuffer {
    make_pyramid(
        Circle {
            radius: width,
            height: 0.0,
        },
        [0.0, 0.0, tip],
        4,
    )
    .merge_with(
        make_pyramid(
            Circle {
                radius: width,
                height: 0.0,
            },
            [0.0, 0.0, tip],
            4,
        )
        .transform_by(Affine3A::from_rotation_x(180_f32.to_radians())),
    )
}

pub(crate) fn make_flat_square_mesh(extent: f32) -> MeshBuffer {
    return make_flat_rect_mesh(extent, extent);
}

pub(crate) fn make_flat_rect_mesh(x_size: f32, y_size: f32) -> MeshBuffer {
    let x = x_size / 2.0;
    let y = y_size / 2.0;
    let positions: Vec<[f32; 3]> = [[-x, -y, 0.], [x, -y, 0.], [x, y, 0.], [-x, y, 0.]]
        .into_iter()
        .cycle()
        .take(8)
        .collect();

    let indices = [0, 1, 2, 0, 2, 3, 4, 6, 5, 4, 7, 6].into_iter().collect();

    let normals: Vec<[f32; 3]> = [[0., 0., 1.]]
        .into_iter()
        .cycle()
        .take(4)
        .chain([[0., 0., -1.]].into_iter().cycle().take(4))
        .collect();

    let uv: Vec<[f32; 2]> = [[0.0, 1.0], [1.0, 1.0], [1.0, 0.0], [0.0, 0.0]]
        .into_iter()
        .cycle()
        .take(8)
        .collect();

    let outline = [0, 1, 1, 2, 2, 3, 3, 0].into_iter().collect();

    return MeshBuffer::new(positions, normals, indices)
        .with_uv(uv)
        .with_outline(outline);
}

pub(crate) fn make_flat_mesh_for_aabb(aabb: Aabb) -> MeshBuffer {
    make_flat_rect_mesh(2.0 * aabb.half_extents.x, 2.0 * aabb.half_extents.y)
        .transform_by(Affine3A::from_translation(aabb.center.into()))
}

pub(crate) fn make_halo_mesh() -> Mesh {
    let inner_ring = 1.0;
    let mid_ring = 1.1 * inner_ring;
    let outer_ring = 1.2 * inner_ring;
    let peak = 0.01;
    let segments = 100u32;
    let gap = 60_f32.to_radians();

    let positions: Vec<[f32; 3]> = make_circles(
        [
            (inner_ring, 0.).into(),
            (mid_ring, peak).into(),
            (outer_ring, 0.).into(),
        ],
        segments,
        gap,
    )
    .collect();

    let colors: Vec<[f32; 4]> = [[1., 1., 1., 1.]]
        .into_iter()
        .cycle()
        .take(2 * segments as usize)
        .chain(
            [[1., 1., 1., 0.]]
                .into_iter()
                .cycle()
                .take(segments as usize),
        )
        .collect();

    let normals: Vec<[f32; 3]> = [[0., 0., 1.]]
        .into_iter()
        .cycle()
        .take(positions.len())
        .collect();

    let indices = Indices::U32(
        [[0u32, segments, segments + 1u32, 0u32, segments + 1u32, 1u32]]
            .into_iter()
            .cycle()
            .enumerate()
            .flat_map(|(cycle, values)| {
                [(cycle as u32, values)]
                    .into_iter()
                    .cycle()
                    .enumerate()
                    .take(segments as usize - 1)
                    .flat_map(|(segment, (cycle, values))| {
                        values.map(|s| cycle * segments + segment as u32 + s)
                    })
            })
            .take(6 * 2 * (segments as usize - 1))
            .chain([
                0,
                2 * segments,
                segments,
                3 * segments - 1,
                segments - 1,
                2 * segments - 1,
            ])
            .collect(),
    );

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.set_indices(Some(indices));
    return mesh;
}

pub(crate) fn make_ring(
    inner_radius: f32,
    outer_radius: f32,
    resolution: usize,
) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = make_circles(
        [
            (inner_radius, 0.).into(),
            (outer_radius, 0.).into(),
        ],
        resolution as u32,
        0.0,
    ).collect();

    let normals: Vec<[f32; 3]> = [[0., 0., 1.0]]
        .into_iter()
        .cycle()
        .take(positions.len())
        .collect();

    let r = resolution as u32;
    let indices = [[0, r, r+1, 0, r+1, 1]]
        .into_iter()
        .cycle()
        .enumerate()
        .flat_map(|(cycle, values)| {
            values.map(|s| s + cycle as u32)
        })
        .take(6 * (resolution - 1))
        .chain([r-1, 2*r-1, r, r-1, r, 0])
        .collect();

    MeshBuffer::new(positions, normals, indices)
}

pub(crate) fn make_icon_halo(
    radius: f32,
    height: f32,
    segments: usize,
) -> MeshBuffer {
    let angle = (360.0/(2.0*segments as f32)).to_radians();
    let p0 = radius * Vec3::X;
    let p1 = Affine3A::from_rotation_z(angle).transform_vector3(p0);
    let width = (p1 - p0).length();
    let mut mesh = make_ring(radius, radius+width/2.0, 32);
    for i in 0..segments {
        mesh = mesh.merge_with(
            make_box(width/2.0, width/2.0, height/2.0)
            .transform_by(Affine3A::from_translation(
                Vec3::new(radius + width/2.0, 0.0, height/2.0)
            ))
            .transform_by(Affine3A::from_rotation_z(i as f32 * 2.0 * angle))
        );
    }

    mesh
}
