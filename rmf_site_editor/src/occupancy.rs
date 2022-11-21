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

use crate::{
    site::Category,
    interaction::VisualCue,
};
use bevy::{
    prelude::*,
    math::{Vec3A, Mat3A},
    render::{primitives::Aabb, mesh::{VertexAttributeValues, PrimitiveTopology, Indices}},
};
use std::collections::HashSet;
use itertools::Itertools;
pub use mapf::occupancy::Cell;

pub struct Grid {
    pub occupied: HashSet<Cell>,
    pub cell_size: f64,
    pub range: GridRange,
    pub floor: f32,
    pub ceiling: f32,
}

#[derive(Clone, Copy, Debug)]
pub struct GridRange {
    min: [i64; 2],
    max: [i64; 2],
}

impl GridRange {
    pub fn new() -> Self {
        GridRange { min: [i64::MAX, i64::MAX], max: [i64::MIN, i64::MIN] }
    }

    pub fn include(&mut self, cell: Cell) {
        self.min = self.min.zip([cell.x, cell.y]).map(|(a, b)| a.min(b));
        self.max = self.min.zip([cell.x, cell.y]).map(|(a, b)| a.max(b));
    }

    pub fn union_with(self, other: GridRange) -> Self {
        GridRange {
            min: self.min.zip(other.min).map(|(a, b)| a.min(b)),
            max: self.max.zip(other.max).map(|(a, b)| a.max(b)),
        }
    }

    pub fn iter(&self) -> impl Iterator<Item = (i64, i64)> {
        (self.min[0]..=self.max[0]).cartesian_product(self.min[1]..=self.max[1])
    }
}

pub struct CalculateGrid {
    /// How large is each cell
    pub cell_size: f64,
    /// Ignore meshes below this height
    pub floor: f32,
    /// Ignore meshes above this height
    pub ceiling: f32,
}

pub fn calculate_grid(
    commands: Commands,
    mut request: EventReader<CalculateGrid>,
    bodies: Query<(Entity, &Handle<Mesh>, &Aabb, &GlobalTransform)>,
    meta: Query<(Option<&Parent>, Option<&Category>, Option<&VisualCue>)>,
    meshes: Res<Assets<Mesh>>,
) {
    if let Some(request) = request.iter().last() {
        let mut occupied: HashSet<Cell> = HashSet::new();
        let mut range = GridRange::new();
        let cell_size = request.cell_size as f32;
        let half_cell_size = cell_size/2.0;
        let floor = request.floor;
        let ceiling = request.ceiling;
        let mid = (floor + ceiling)/2.0;
        let half_height = (ceiling - floor)/2.0;

        let physical_entities = collect_physical_entities(&bodies, &meta);
        for e in &physical_entities {
            let (_, mesh, aabb, tf) = match bodies.get(*e) {
                Ok(body) => body,
                Err(_) => continue,
            };

            let body_range = match grid_range_of_aabb(aabb, tf, cell_size, floor, ceiling) {
                Some(range) => range,
                None => continue,
            };

            range = range.union_with(body_range);

            if let Some(mesh) = meshes.get(mesh) {
                if mesh.primitive_topology() != PrimitiveTopology::TriangleList {
                    continue;
                }

                let positions = match mesh.attribute(Mesh::ATTRIBUTE_POSITION) {
                    Some(VertexAttributeValues::Float32x3(positions)) => positions,
                    _ => continue,
                };

                let indices = match mesh.indices() {
                    Some(Indices::U32(indices)) => indices,
                    _ => {
                        println!("Unexpected index set for mesh of {e:?}:\n{:?}", mesh.indices());
                        continue;
                    }
                };

                for (x, y) in range.iter() {
                    let cell = Cell::new(x, y);
                    if occupied.contains(&cell) {
                        // No reason to check this cell since we already know
                        // that it is occupied.
                        continue;
                    }

                    let b = Aabb {
                        center: Vec3A::new(
                            cell_size * (x as f32 + 0.5),
                            cell_size * (y as f32 + 0.5),
                            mid,
                        ),
                        half_extents: Vec3A::new(
                            half_cell_size,
                            half_cell_size,
                            half_height
                        )
                    };

                    if mesh_intersects_box(&b, positions, indices) {
                        occupied.insert(cell);
                    }
                }
            }
        }
    }
}

fn collect_physical_entities(
    meshes: &Query<(Entity, &Handle<Mesh>, &Aabb, &GlobalTransform)>,
    meta: &Query<(Option<&Parent>, Option<&Category>, Option<&VisualCue>)>,
) -> Vec<Entity> {
    let mut physical_entities = Vec::new();
    for (e, _, _, _) in meshes {
        let is_physical = loop {
            let mut e_meta = e;
            if let Ok((parent, category, cue)) = meta.get(e_meta) {
                if cue.is_some() {
                    // This is a visual cue, making it non-physical
                    break false;
                }

                if let Some(category) = category {
                    break category.is_physical();
                }

                if let Some(parent) = parent {
                    e_meta = parent.get();
                } else {
                    // There is no parent and we have not determined a
                    // category for this mesh, so let's assume it is not
                    // physical
                    break false;
                }
            } else {
                // Should this ever happen?
                break false;
            }
        };

        if is_physical {
            physical_entities.push(e);
        }
    }

    physical_entities
}

fn grid_range_of_aabb(
    aabb: &Aabb,
    tf: &GlobalTransform,
    cell_size: f32,
    floor: f32,
    ceiling: f32,
) -> Option<GridRange> {
    let mut range = GridRange::new();
    let mut is_below = false;
    let mut is_inside = false;
    let mut is_above = false;
    for x in [-1_f32, 1_f32] {
        for y in [-1_f32, 1_f32] {
            for z in [-1_f32, 1_f32] {
                let m = Mat3A::from_diagonal(Vec3::new(x, y, z));
                let corner = tf.affine().transform_point3a(
                    aabb.center + m*aabb.half_extents
                );

                if corner.z < floor {
                    is_below = true;
                } else if ceiling < corner.z {
                    is_above = true;
                } else {
                    is_inside = true;
                }

                let cell = Cell::new(
                    (corner.x / cell_size).floor() as i64,
                    (corner.y / cell_size).floor() as i64,
                );

                range.include(cell);
            }
        }
    }

    if is_inside {
        return Some(range);
    }

    if is_above && is_below {
        return Some(range);
    }

    return None;
}

fn mesh_intersects_box(
    b: &Aabb,
    positions: &Vec<[f32; 3]>,
    indices: &Vec<u32>,
) {
    // This uses the algorithm described here:
    // https://fileadmin.cs.lth.se/cs/Personal/Tomas_Akenine-Moller/code/tribox_tam.pdf
    for t_index in 0..indices.len()/3 {
        let p0: Vec3A = positions[3*t_index + 0].into();
        let p1: Vec3A = positions[3*t_index + 1].into();
        let p2: Vec3A = positions[3*t_index + 2].into();

        let p0 = p0 - b.center;
        let p1 = p1 - b.center;
        let p2 = p2 - b.center;
        let points = [p0, p1, p2];
        for i in 0..3 {
            let mut sorted = points.map(|p| p[i]);
            sorted.sort_by(|a, b| a.partial_cmp(&b).unwrap());
            if b.half_extents[i] < sorted[0] {
                return false;
            }

            if sorted[2] < -b.half_extents[i] {
                return false;
            }
        }


    }
}
