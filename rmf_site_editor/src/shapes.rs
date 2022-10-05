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
    render::mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
};
use rmf_site_format::Angle;

pub(crate) struct MeshBuffer {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl MeshBuffer {
    pub(crate) fn transform_by(mut self, tf: Affine3A) -> Self {
        for p in &mut self.positions {
            *p = tf.transform_point3((*p).into()).into();
        }

        for n in &mut self.normals {
            *n = tf.transform_vector3((*n).into()).into();
        }

        self
    }

    pub(crate) fn merge_with(mut self, mut other: Self) -> Self {
        let offset = self.positions.len();
        self.indices.extend(other.indices.into_iter().map(|i| i + offset as u32));
        self.positions.extend(other.positions.into_iter());
        self.normals.extend(other.normals.into_iter());
        self
    }

    pub(crate) fn merge_into(self, mesh: &mut Mesh) {
        let offset = mesh.attribute(Mesh::ATTRIBUTE_POSITION).map(|a| a.len());
        if let Some(offset) = offset {
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
        } else {
            // The mesh currently has no positions in it (and should therefore have no normals or indices either)
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, self.positions);
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, self.normals);
            mesh.set_indices(Some(Indices::U32(self.indices)));
        }
    }
}

impl From<MeshBuffer> for Mesh {
    fn from(partial: MeshBuffer) -> Self {
        let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
        mesh.set_indices(Some(Indices::U32(partial.indices)));
        mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, partial.positions);
        mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, partial.normals);
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
                let theta =
                    (i as f32) / (resolution as f32 - 1.) * (std::f32::consts::TAU - gap);
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

    return MeshBuffer {
        positions,
        normals,
        indices,
    };
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

    return MeshBuffer {
        positions,
        normals,
        indices,
    };
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

    return MeshBuffer {
        positions,
        normals,
        indices,
    };
}

fn make_cone(circle: Circle, peak: [f32; 3], resolution: u32) -> MeshBuffer {
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

    return MeshBuffer {
        positions,
        normals,
        indices,
    };
}

pub(crate) fn make_bottom_circle(circle: Circle, resolution: u32) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = make_circles([circle], resolution, 0.)
        .take(resolution as usize - 1) // skip the vertex which would close the circle
        .chain([[0., 0., circle.height]].into_iter())
        .collect();

    let peak = positions.len() as u32 - 1;
    let indices: Vec<u32> = (0..resolution - 1)
        .into_iter()
        .flat_map(|i| [i, peak, i + 1].into_iter())
        .chain([resolution - 1, peak, 0])
        .collect();

    let normals: Vec<[f32; 3]> = [[0., 0., -1.]]
        .into_iter()
        .cycle()
        .take(positions.len())
        .collect();

    return MeshBuffer {
        positions,
        normals,
        indices,
    };
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
    let half_handle_width = handle_width/2.0;
    let half_tip_width = tip_width/2.0;
    let positions: Vec<[f32; 3]> = vec![
        [0.0, half_handle_width, 0.0], // 0
        [0.0, -half_handle_width, 0.0], // 1
        [handle_length, -half_handle_width, 0.0], // 2
        [handle_length, half_handle_width, 0.0], // 3
        [handle_length, half_tip_width, 0.0], // 4
        [handle_length, -half_tip_width, 0.0], // 5
        [handle_length + tip_length, 0.0, 0.0], // 6
    ];

    let normals: Vec<[f32; 3]> = {
        let mut normals = Vec::new();
        normals.resize(positions.len(), [0.0, 0.0, 1.0]);
        normals
    };

    let indices: Vec<u32> = vec![
        0, 1, 3,
        1, 2, 3,
        4, 5, 6,
    ];

    MeshBuffer {positions, normals, indices}
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
    let center = (start + stop)/2.0;
    let dp = stop - start;
    let yaw = dp.y.atan2(dp.x);

    flat_arrow_mesh(handle_length, handle_width, tip_length, tip_width)
        .transform_by(
            Affine3A::from_scale_rotation_translation(
                Vec3::new(1.0, 1.0, 1.0),
                Quat::from_rotation_z(yaw),
                start,
            )
        )
}

pub(crate) fn flat_arc(
    outer_radius: f32,
    inner_thickness: f32,
    angle: Angle,
    vertices_per_degree: f32,
) -> MeshBuffer {
    let resolution = (angle.degrees() * vertices_per_degree) as u32;
    let positions: Vec<[f32; 3]> = make_circles(
        [
            (outer_radius - inner_thickness, 0.).into(),
            (outer_radius, 0.).into(),
        ],
        resolution,
        std::f32::consts::TAU - angle.radians(),
    ).collect();

    let normals: Vec<[f32; 3]> = {
        let mut normals = Vec::new();
        normals.resize(positions.len(), [0.0, 0.0, 1.0]);
        normals
    };

    let indices: Vec<u32> = [[0, resolution, resolution + 1, 0, resolution + 1, 1]]
        .into_iter()
        .cycle()
        .enumerate()
        .flat_map(|(segment, values)| {
            values.map(|s| segment as u32 + s)
        })
        .take(6 * (resolution as usize - 1))
        .collect();

    MeshBuffer { positions, normals, indices }
}

pub(crate) fn line_stroke_mesh(
    start: Vec3,
    end: Vec3,
    thickness: f32,
) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = vec![
        [-0.5, -0.5, 0.], // 0
        [0.5, -0.5, 0.], // 1
        [0.5, 0.5, 0.], // 2
        [-0.5, 0.5, 0.], // 3
    ];

    let normals: Vec<[f32; 3]> = {
        let mut normals = Vec::new();
        normals.resize(positions.len(), [0.0, 0.0, 1.0]);
        normals
    };

    let indices: Vec<u32> = vec![0, 1, 2, 0, 2, 3];

    let center = (start + end) / 2.0;
    let dp = end - start;
    let yaw = dp.y.atan2(dp.x);

    MeshBuffer { positions, normals, indices }.transform_by(
        Affine3A::from_scale_rotation_translation(
            Vec3::new(dp.length(), thickness, 1.),
            Quat::from_rotation_z(yaw),
            center,
        )
    )
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
        Affine3A::from_translation([(1. - lens_hood_protrusion) * scale, 0., 0.].into())
            * Affine3A::from_rotation_y(90_f32.to_radians())
            * Affine3A::from_rotation_z(45_f32.to_radians()),
    )
    .merge_into(&mut mesh);

    mesh
}

pub(crate) fn make_flat_square_mesh(extent: f32) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = [
        [-extent, -extent, 0.],
        [extent, -extent, 0.],
        [extent, extent, 0.],
        [-extent, extent, 0.],
    ]
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

    return MeshBuffer {
        positions,
        normals,
        indices,
    };
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
