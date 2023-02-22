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
    interaction::ComputedVisualCue,
    shapes::*,
    site::{Category, LevelProperties, SiteAssets, SiteProperties, PASSIVE_LANE_HEIGHT},
};
use bevy::{
    math::{swizzles::*, Affine3A, Mat3A, Vec2, Vec3A},
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology, VertexAttributeValues},
        primitives::Aabb,
    },
};
use itertools::Itertools;
use std::{
    collections::{HashMap, HashSet},
    time::Instant,
};

pub struct OccupancyPlugin;

impl Plugin for OccupancyPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CalculateGrid>().add_system(calculate_grid);
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Cell {
    pub x: i64,
    pub y: i64,
}

impl Cell {
    /// Make a new cell from a pair of indices.
    pub fn new(x: i64, y: i64) -> Self {
        Self { x, y }
    }

    /// Get the cell that this point is inside of. Points that are perfectly on
    /// the edge between two cells will be biased towards the cell with the
    /// higher index value.
    pub fn from_point(p: Vec2, cell_size: f32) -> Self {
        Self {
            x: (p.x / cell_size).floor() as i64,
            y: (p.y / cell_size).floor() as i64,
        }
    }

    /// Get the point in the center of the cell.
    pub fn to_center_point(&self, cell_size: f32) -> Vec2 {
        Vec2::new(
            cell_size * (self.x as f32 + 0.5),
            cell_size * (self.y as f32 + 0.5),
        )
    }

    /// Get a new cell that is the same as this one, but shifted in x and y by
    /// the given values.
    pub fn shifted(&self, x: i64, y: i64) -> Self {
        Self {
            x: self.x + x,
            y: self.y + y,
        }
    }
}

#[derive(Component)]
pub struct Grid {
    pub occupied: HashSet<Cell>,
    pub cell_size: f32,
    pub floor: f32,
    pub ceiling: f32,
    pub range: GridRange,
}

#[derive(Clone, Copy, Debug)]
pub struct GridRange {
    min: [i64; 2],
    max: [i64; 2],
}

impl GridRange {
    pub fn new() -> Self {
        GridRange {
            min: [i64::MAX, i64::MAX],
            max: [i64::MIN, i64::MIN],
        }
    }

    pub fn include(&mut self, cell: Cell) {
        self.min[0] = self.min[0].min(cell.x);
        self.min[1] = self.min[1].min(cell.y);
        self.max[0] = self.max[0].max(cell.x);
        self.max[1] = self.max[1].max(cell.y);
    }

    pub fn min_cell(&self) -> Cell {
        Cell::new(self.min[0], self.min[1])
    }

    pub fn max_cell(&self) -> Cell {
        Cell::new(self.max[0], self.max[1])
    }

    pub fn union_with(mut self, other: GridRange) -> Self {
        self.include(other.min_cell());
        self.include(other.max_cell());
        self
    }

    pub fn iter(&self) -> impl Iterator<Item = (i64, i64)> {
        (self.min[0]..=self.max[0]).cartesian_product(self.min[1]..=self.max[1])
    }
}

pub struct CalculateGrid {
    /// How large is each cell
    pub cell_size: f32,
    /// Ignore meshes below this height
    pub floor: f32,
    /// Ignore meshes above this height
    pub ceiling: f32,
}

enum Group {
    Level(Entity),
    Site(Entity),
    None,
}

fn calculate_grid(
    mut commands: Commands,
    mut request: EventReader<CalculateGrid>,
    bodies: Query<(Entity, &Handle<Mesh>, &Aabb, &GlobalTransform)>,
    meta: Query<(
        Option<&Parent>,
        Option<&Category>,
        Option<&ComputedVisualCue>,
    )>,
    parents: Query<&Parent>,
    levels: Query<Entity, With<LevelProperties>>,
    sites: Query<(), With<SiteProperties>>,
    mut meshes: ResMut<Assets<Mesh>>,
    assets: Res<SiteAssets>,
    grids: Query<Entity, With<Grid>>,
) {
    if let Some(request) = request.iter().last() {
        let start_time = Instant::now();
        // let mut occupied: HashSet<Cell> = HashSet::new();
        let mut occupied: HashMap<Entity, HashSet<Cell>> = HashMap::new();
        let mut range = GridRange::new();
        let cell_size = request.cell_size as f32;
        let half_cell_size = cell_size / 2.0;
        let floor = request.floor;
        let ceiling = request.ceiling;
        let mid = (floor + ceiling) / 2.0;
        let half_height = (ceiling - floor) / 2.0;
        let levels_of_sites = get_levels_of_sites(&levels, &parents);

        let physical_entities = collect_physical_entities(&bodies, &meta);
        println!("Checking {:?} physical entities", physical_entities.len());
        for e in &physical_entities {
            let (_, mesh, aabb, tf) = match bodies.get(*e) {
                Ok(body) => body,
                Err(_) => continue,
            };

            let e_group = match get_group(*e, &parents, &levels, &sites) {
                Group::Level(e) | Group::Site(e) => e,
                Group::None => continue,
            };

            let group_occupied = occupied.entry(e_group).or_default();

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
                        println!(
                            "Unexpected index set for mesh of {e:?}:\n{:?}",
                            mesh.indices()
                        );
                        continue;
                    }
                };

                for (x, y) in body_range.iter() {
                    let cell = Cell::new(x, y);
                    if group_occupied.contains(&cell) {
                        // No reason to check this cell since we already know
                        // that it is occupied.
                        continue;
                    }

                    let b = Aabb {
                        center: Cell::new(x, y)
                            .to_center_point(cell_size)
                            .extend(mid)
                            .into(),
                        half_extents: Vec3A::new(half_cell_size, half_cell_size, half_height),
                    };

                    if mesh_intersects_box(&b, positions, indices, tf) {
                        group_occupied.insert(cell);
                    }
                }
            }
        }

        let finish_time = Instant::now();
        let delta = finish_time - start_time;
        println!("Occupancy calculation time: {}", delta.as_secs_f32());

        for grid in &grids {
            commands.entity(grid).despawn_recursive();
        }

        for (site, levels) in levels_of_sites {
            let site_occupancy = occupied.get(&site).cloned().unwrap_or_default();
            for level in levels {
                let level_occupied = occupied.entry(level).or_default();
                for cell in &site_occupancy {
                    level_occupied.insert(*cell);
                }
            }
        }

        for level in &levels {
            let mut mesh = MeshBuffer::empty();
            let level_occupied = match occupied.remove(&level) {
                Some(o) => o,
                None => continue,
            };
            for cell in &level_occupied {
                let p = Vec3::new(
                    cell_size * (cell.x as f32 + 0.5),
                    cell_size * (cell.y as f32 + 0.5),
                    PASSIVE_LANE_HEIGHT / 2.0,
                );
                mesh = mesh.merge_with(
                    make_flat_square_mesh(cell_size).transform_by(Affine3A::from_translation(p)),
                );
            }

            commands.entity(level).add_children(|level| {
                level
                    .spawn(PbrBundle {
                        mesh: meshes.add(mesh.into()),
                        material: assets.occupied_material.clone(),
                        ..default()
                    })
                    .insert(Grid {
                        occupied: level_occupied,
                        cell_size,
                        floor,
                        ceiling,
                        range,
                    });
            });
        }
    }
}

fn get_levels_of_sites(
    levels: &Query<Entity, With<LevelProperties>>,
    parents: &Query<&Parent>,
) -> HashMap<Entity, Vec<Entity>> {
    let mut levels_of_sites: HashMap<Entity, Vec<Entity>> = HashMap::new();
    for level in levels {
        if let Ok(parent) = parents.get(level) {
            levels_of_sites.entry(parent.get()).or_default().push(level);
        }
    }

    levels_of_sites
}

fn get_group(
    e: Entity,
    parents: &Query<&Parent>,
    levels: &Query<Entity, With<LevelProperties>>,
    sites: &Query<(), With<SiteProperties>>,
) -> Group {
    let mut e_meta = e;
    loop {
        if levels.contains(e_meta) {
            return Group::Level(e_meta);
        }

        if sites.contains(e_meta) {
            return Group::Site(e_meta);
        }

        if let Ok(parent) = parents.get(e_meta) {
            e_meta = parent.get();
        } else {
            return Group::None;
        }
    }
}

fn collect_physical_entities(
    meshes: &Query<(Entity, &Handle<Mesh>, &Aabb, &GlobalTransform)>,
    meta: &Query<(
        Option<&Parent>,
        Option<&Category>,
        Option<&ComputedVisualCue>,
    )>,
) -> Vec<Entity> {
    let mut physical_entities = Vec::new();
    for (e, _, _, _) in meshes {
        let mut e_meta = e;
        let is_physical = loop {
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
                let corner = tf
                    .affine()
                    .transform_point3a(aabb.center + m * aabb.half_extents);

                if corner.z < floor {
                    is_below = true;
                } else if ceiling < corner.z {
                    is_above = true;
                } else {
                    is_inside = true;
                }

                let cell = Cell::from_point(corner.xy(), cell_size);

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
    mesh_tf: &GlobalTransform,
) -> bool {
    for t_index in 0..indices.len() / 3 {
        let p0: Vec3A = positions[indices[3 * t_index + 0] as usize].into();
        let p1: Vec3A = positions[indices[3 * t_index + 1] as usize].into();
        let p2: Vec3A = positions[indices[3 * t_index + 2] as usize].into();
        let points = [
            mesh_tf.affine().transform_point3a(p0),
            mesh_tf.affine().transform_point3a(p1),
            mesh_tf.affine().transform_point3a(p2),
        ];
        if triangle_intersects_box(b, points) {
            return true;
        }
    }

    return false;
}

fn triangle_intersects_box(b: &Aabb, points: [Vec3A; 3]) -> bool {
    // This uses the algorithm described here:
    // https://fileadmin.cs.lth.se/cs/Personal/Tomas_Akenine-Moller/code/tribox_tam.pdf
    let points = points.map(|p| p - b.center);

    // Test AABB of grid cell vs AABB of triangle
    for i in 0..=2 {
        let mut sorted = points.map(|p| p[i]);
        sorted.sort_by(|a, b| a.total_cmp(&b));
        if b.half_extents[i] < sorted[0] {
            return false;
        }

        if sorted[2] < -b.half_extents[i] {
            return false;
        }
    }

    let n = match (points[2] - points[0])
        .cross(points[1] - points[0])
        .try_normalize()
    {
        Some(n) => n,
        None => {
            // This triange has no volume, so lets ignore it.
            return false;
        }
    };

    // Test triangle plane against bounding box
    let triangle_dist = n.dot(points[0]).abs();
    let box_reach = b.half_extents.dot(n.abs());
    if box_reach < triangle_dist {
        return false;
    }

    let edges = [
        points[1] - points[0],
        points[2] - points[1],
        points[0] - points[2],
    ];
    for (i, j) in (0..=2).into_iter().cartesian_product(0..=2) {
        let a = unit_vec(i).cross(edges[j]);
        let mut sorted = points.map(|p| a.dot(p));
        sorted.sort_by(|a, b| a.total_cmp(b));
        let r = b.half_extents.dot(a.abs());
        if r < sorted[0] || sorted[2] < -r {
            return false;
        }
    }

    return true;
}

fn unit_vec(axis: usize) -> Vec3A {
    let mut v = Vec3A::ZERO;
    v[axis] = 1.0;
    v
}
