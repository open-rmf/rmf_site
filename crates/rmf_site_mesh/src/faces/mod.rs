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

use bevy_color::{Color};
use bevy_math::prelude::*;
use bevy_math::{Affine3A, primitives};
use bevy_render::prelude::*;

use bevy_render::primitives::Aabb;
use std::collections::{BTreeMap, HashMap};


use crate::*;

#[derive(Clone, Copy, Debug)]
pub struct OffsetCircle {
    pub radius: f32,
    pub height: f32,
}

impl OffsetCircle {
    pub fn flip_height(mut self) -> Self {
        self.height = -self.height;
        self
    }
}

impl From<(f32, f32)> for OffsetCircle {
    fn from((radius, height): (f32, f32)) -> Self {
        Self { radius, height }
    }
}

pub fn make_circles(
    circles: impl IntoIterator<Item = OffsetCircle>,
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

pub fn make_smooth_wrap(circles: [OffsetCircle; 2], resolution: u32) -> MeshBuffer {
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

pub fn make_wall_mesh(
    p_start: Vec3,
    p_end: Vec3,
    thickness: f32,
    height: f32,
    texture_height: Option<f32>,
    texture_width: Option<f32>,
) -> MeshBuffer {
    let dp = p_end - p_start;
    let length = dp.length();
    let yaw = dp.y.atan2(dp.x);
    let center = (p_start + p_end) / 2.0;
    let texture_height = texture_height.unwrap_or(height);
    let texture_width = texture_width.unwrap_or(1.0);

    // The default UV coordinates made by bevy do not work well for walls,
    // so we customize them here
    let uv = vec![
        // Top
        [0., 0.], // 0
        [0., 0.], // 1
        [0., 0.], // 2
        [0., 0.], // 3
        // Bottom
        [0., height / texture_height], // 4
        [0., height / texture_height], // 5
        [0., height / texture_height], // 6
        [0., height / texture_height], // 7
        // Right
        [length / texture_width, height / texture_height], // 8
        [0., height / texture_height],                     // 9
        [0., 0.],                                          // 10
        [length / texture_width, 0.],                      // 11
        // Left
        [0., 0.],                                          // 12
        [length / texture_width, 0.],                      // 13
        [length / texture_width, height / texture_height], // 14
        [0., height / texture_height],                     // 15
        // Front
        [0., height / texture_height],                     // 16
        [length / texture_width, height / texture_height], // 17
        [length / texture_width, 0.],                      // 18
        [0., 0.],                                          // 19
        // Back
        [length / texture_width, 0.],                      // 20
        [0., 0.],                                          // 21
        [0., height / texture_height],                     // 22
        [length / texture_width, height / texture_height], // 23
    ];
    make_cuboid(length, thickness, height)
        .with_uv(uv)
        .transform_by(
            Affine3A::from_translation(Vec3::new(center.x, center.y, height / 2.0))
                * Affine3A::from_rotation_z(yaw),
        )
}

pub fn make_top_circle(circle: OffsetCircle, resolution: u32) -> MeshBuffer {
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

pub fn make_bottom_circle(circle: OffsetCircle, resolution: u32) -> MeshBuffer {
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

pub fn make_flat_disk(circle: OffsetCircle, resolution: u32) -> MeshBuffer {
    make_top_circle(circle, resolution).merge_with(make_bottom_circle(circle, resolution))
}

pub fn make_cylinder(height: f32, radius: f32) -> MeshBuffer {
    let top_circle = OffsetCircle {
        height: height / 2.0,
        radius,
    };
    let mid_circle = OffsetCircle {
        height: 0.0,
        radius,
    };
    let bottom_circle = OffsetCircle {
        height: -height / 2.0,
        radius,
    };
    let resolution = 32;
    make_smooth_wrap([top_circle, bottom_circle], resolution)
        .merge_with(
            make_bottom_circle(mid_circle, resolution)
                .transform_by(Affine3A::from_translation([0.0, 0., -height / 2.0].into())),
        )
        .merge_with(make_bottom_circle(mid_circle, resolution).transform_by(
            Affine3A::from_translation([0., 0., height / 2.0].into())
                * Affine3A::from_rotation_x(180_f32.to_radians()),
        ))
}

pub fn flat_arc(
    pivot: Vec3,
    outer_radius: f32,
    inner_thickness: f32,
    initial_angle: impl Into<Radians>,
    sweep: impl Into<Radians>,
    vertices_per_degree: f32,
) -> MeshBuffer {
    let initial_angle: Radians = initial_angle.into();
    let sweep = sweep.into();
    let (initial_angle, sweep) = if sweep < 0.0 {
        (initial_angle + sweep, -sweep)
    } else {
        (initial_angle, sweep)
    };

    let resolution = (Degrees::from(sweep) * vertices_per_degree).0 as u32;
    let positions: Vec<[f32; 3]> = make_circles(
        [
            (outer_radius - inner_thickness, 0.).into(),
            (outer_radius, 0.).into(),
        ],
        resolution,
        std::f32::consts::TAU - *sweep,
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
            Quat::from_rotation_z(*initial_angle),
            pivot,
        ))
}

pub fn line_stroke_mesh(start: Vec3, end: Vec3, thickness: f32) -> MeshBuffer {
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

pub fn line_stroke_away_from(
    start: Vec3,
    direction: Radians,
    length: f32,
    thickness: f32,
) -> MeshBuffer {
    let end = start
        + Affine3A::from_rotation_z(direction.0).transform_vector3(Vec3::new(length, 0.0, 0.0));

    line_stroke_mesh(start, end, thickness)
}

pub fn make_physical_camera_mesh() -> Mesh {
    let scale = 0.1;
    let lens_hood_protrusion = 0.8;

    // Main body
    let mut mesh: Mesh = primitives::Cuboid::new(scale, scale, scale).into();
    mesh.remove_attribute(Mesh::ATTRIBUTE_UV_0);

    // Outside of the lens hood
    make_pyramid(
        OffsetCircle {
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
        OffsetCircle {
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

pub fn make_diamond(tip: f32, width: f32) -> MeshBuffer {
    make_pyramid(
        OffsetCircle {
            radius: width,
            height: 0.0,
        },
        [0.0, 0.0, tip],
        4,
    )
    .merge_with(
        make_pyramid(
            OffsetCircle {
                radius: width,
                height: 0.0,
            },
            [0.0, 0.0, tip],
            4,
        )
        .transform_by(Affine3A::from_rotation_x(180_f32.to_radians())),
    )
}

pub fn make_flat_square_mesh(extent: f32) -> MeshBuffer {
    return make_flat_rect_mesh(extent, extent);
}

pub fn make_flat_rect_mesh(x_size: f32, y_size: f32) -> MeshBuffer {
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

pub fn make_flat_mesh_for_aabb(aabb: Aabb) -> MeshBuffer {
    make_flat_rect_mesh(2.0 * aabb.half_extents.x, 2.0 * aabb.half_extents.y)
        .transform_by(Affine3A::from_translation(aabb.center.into()))
}

pub fn make_ring(inner_radius: f32, outer_radius: f32, resolution: usize) -> MeshBuffer {
    let positions: Vec<[f32; 3]> = make_circles(
        [(inner_radius, 0.).into(), (outer_radius, 0.).into()],
        resolution as u32,
        0.0,
    )
    .collect();

    let normals: Vec<[f32; 3]> = [[0., 0., 1.0]]
        .into_iter()
        .cycle()
        .take(positions.len())
        .collect();

    let r = resolution as u32;
    let indices = [[0, r, r + 1, 0, r + 1, 1]]
        .into_iter()
        .cycle()
        .enumerate()
        .flat_map(|(cycle, values)| values.map(|s| s + cycle as u32))
        .take(6 * (resolution - 1))
        .chain([r - 1, 2 * r - 1, r, r - 1, r, 0])
        .collect();

    MeshBuffer::new(positions, normals, indices)
}

pub fn make_location_icon(radius: f32, height: f32, segments: usize) -> MeshBuffer {
    let height = 2.0 * height;
    let angle = (360.0 / (2.0 * segments as f32)).to_radians();
    let p0 = radius * Vec3::X;
    let p1 = Affine3A::from_rotation_z(angle).transform_vector3(p0);
    let width = (p1 - p0).length();
    make_flat_square_mesh(width).transform_by(Affine3A::from_translation(Vec3::new(
        radius + width / 2.0,
        0.0,
        height / 2.0,
    )))
}

pub fn make_closed_path_outline(mut initial_positions: Vec<[f32; 3]>) -> MeshBuffer {
    let num_positions = initial_positions.len() as u32;
    let mut positions = Vec::new();
    let mut normals = Vec::new();
    let mut uv = Vec::new();
    let mut indices = Vec::new();
    let z2 = [0.0, 0.0];

    // Close the loop by repeating the first and last values at the end and start, respectively
    if let Some(pf) = initial_positions.last() {
        let pf = *pf;
        if let Some(pi) = initial_positions.first() {
            let pi = *pi;
            initial_positions.push(pi);
        }
        initial_positions.insert(0, pf);
    }

    // for (i, [p0, p1, p2]) in initial_positions.array_windows::<3>().enumerate() {
    for (i, window) in initial_positions.windows(3).enumerate() {
        let i = i as u32;
        let p0 = window[0];
        let p1 = window[1];
        let p2 = window[2];

        let p = p1;
        let p0 = Vec3::new(p0[0], p0[1], 0.0);
        let p1 = Vec3::new(p1[0], p1[1], 0.0);
        let p2 = Vec3::new(p2[0], p2[1], 0.0);
        let v0 = (p1 - p0).normalize_or_zero();
        let v1 = (p2 - p1).normalize_or_zero();

        // n: normal
        let n = Vec3::Z;
        let u = n.cross(v0).normalize();
        let w = n.cross(v1).normalize();

        // b: bisector
        let b = match (u + w).try_normalize() {
            Some(b) => b,
            None => {
                // This means that u and w are pointing in opposite directions,
                // so the next vertex is in a perfect 180 back towards the
                // previous vertex. We can simply use v0 as the bisecting
                // vector.
                v0
            }
        };

        positions.extend([p, p, p, p, p, p, p, p]);
        normals.extend([u, -u, w, -w, b, -b, n, -n].map(Into::<[f32; 3]>::into));
        uv.extend([z2, z2, z2, z2, z2, z2, z2, z2]);

        let u0 = 0;
        let u1 = 1;
        let w0 = 2;
        let w1 = 3;
        let b0 = 4;
        let b1 = 5;
        let n0 = 6;
        let n1 = 7;
        let i_delta = 8;

        // Current base index
        let c = i_delta * i;
        // Next base index
        let f = if i == num_positions - 1 {
            // We have reached the last iteration so we should wrap around and
            // connect to the first set of vertices.
            0
        } else {
            i_delta * (i + 1)
        };

        if w.cross(b).dot(n) < 0.0 {
            // left turn
            indices.extend([
                c + u1,
                c + b1,
                c + n0,
                c + b1,
                c + w1,
                c + n0,
                c + u1,
                c + n1,
                c + b1,
                c + b1,
                c + n1,
                c + w1,
            ]);
        } else {
            // right turn
            indices.extend([
                c + u0,
                c + n0,
                c + b0,
                c + b0,
                c + n0,
                c + w0,
                c + u0,
                c + b0,
                c + n1,
                c + b0,
                c + w0,
                c + n1,
            ]);
        }

        indices.extend([
            c + w0,
            c + n0,
            f + n0,
            c + w0,
            f + n0,
            f + u0,
            c + w1,
            f + n0,
            c + n0,
            c + w1,
            f + u1,
            f + n0,
            c + w0,
            f + u0,
            f + n1,
            c + w0,
            f + n1,
            c + n1,
            c + w1,
            f + n1,
            f + u1,
            c + w1,
            c + n1,
            f + n1,
        ]);
    }

    MeshBuffer::new(positions, normals, indices)
        .with_uv(uv)
        .copy_outline_normals()
}

const X_AXIS_COLOR: Color = Color::srgb(1.0, 0.2, 0.2);
const Y_AXIS_COLOR: Color = Color::srgb(0.2, 1.0, 0.2);
const NEG_X_AXIS_COLOR: Color = Color::srgb(0.5, 0.0, 0.0);
const NEG_Y_AXIS_COLOR: Color = Color::srgb(0.0, 0.5, 0.0);

const POLYLINE_SEPARATOR: Vec3 = Vec3::splat(std::f32::NAN);

pub fn make_finite_grid(
    scale: f32,
    count: u32,
    color: Color,
    weights: BTreeMap<u32, f32>,
) -> Vec<(BoxedPolyline3d, PolylineMaterial)> {
    let d_max = count as f32 * scale;
    let depth_bias = -0.0001;
    let perspective = true;

    let make_point = |i, j, d, w| {
        let mut p = Vec3::ZERO;
        p[i] = w;
        p[j] = d;
        p
    };

    let make_points = |i, j, d| [make_point(i, j, d, d_max), make_point(i, j, d, -d_max)];

    let mut vec_of_lines: HashMap<u32, Vec<Vec3>> = HashMap::new();

    let mut result = {
        let Some(width) = weights.values().last().copied() else {
            return Vec::new();
        };
        let mut axes: Vec<(BoxedPolyline3d, PolylineMaterial)> = Vec::new();

        for (sign, x_axis_color, y_axis_color) in [
            (1.0, X_AXIS_COLOR, Y_AXIS_COLOR),
            (-1.0, NEG_X_AXIS_COLOR, NEG_Y_AXIS_COLOR),
        ] {
            for (i, j, color) in [(0, 1, x_axis_color), (1, 0, y_axis_color)] {
                let p0 = Vec3::ZERO;
                let p1 = make_point(i, j, 0.0, sign * d_max);
                let polyline: BoxedPolyline3d = BoxedPolyline3d::new([p0, p1]);
                let material = PolylineMaterial {
                    width,
                    color: color.into(),
                    depth_bias,
                    perspective,
                };
                axes.push((polyline, material));
            }
        }

        axes
    };

    for n in 1..=count {
        let d = n as f32 * scale;
        let polyline = {
            let Some(weight_key) = weights.keys().rev().find(|k| n % **k == 0) else {
                continue;
            };
            vec_of_lines.entry(*weight_key).or_insert(Vec::default())
        };

        for (i, j) in [(0, 1), (1, 0)] {
            polyline.extend(make_points(i, j, d));
            polyline.push(POLYLINE_SEPARATOR);
            polyline.extend(make_points(i, j, -d));
            polyline.push(POLYLINE_SEPARATOR);
        }
    }

    result.extend(vec_of_lines.into_iter().map(|(n, polyline)| {
        let width = *weights.get(&n).unwrap();
        let material = PolylineMaterial {
            width,
            color: color.into(),
            depth_bias,
            perspective,
        };
        (BoxedPolyline3d::new(polyline), material)
    }));
    result
}

pub fn make_metric_finite_grid(
    scale: f32,
    count: u32,
    color: Color,
) -> Vec<(BoxedPolyline3d, PolylineMaterial)> {
    make_finite_grid(scale, count, color, [(1, 0.5), (5, 1.0), (10, 1.5)].into())
}
