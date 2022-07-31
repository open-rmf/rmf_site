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
    site_map::SiteAssets,
};
use bevy::{
    prelude::*,
    render::mesh::{
        Mesh, PrimitiveTopology, Indices, VertexAttributeValues,
    }
};
use bevy_mod_picking::{
    PickingRaycastSet,
    PickingSystem,
};
use bevy_mod_raycast::{
    Intersection,
};
use std::{fmt::Debug, hash::Hash};

#[derive(Debug, Component)]
pub struct Cursor {
    halo: Entity,
    dagger: Entity,
    vertex: Entity,
}

pub struct InteractionAssets {
    pub halo_mesh: Handle<Mesh>,
    pub dagger_mesh: Handle<Mesh>,
}

/// Used to mark halo meshes so their rotations can be animated
#[derive(Debug, Component)]
pub struct Spinning {
    period: f32,
}

impl Default for Spinning {
    fn default() -> Self {
        Self{period: 2.}
    }
}

#[derive(Debug, Component)]
pub struct Bobbing {
    period: f32,
    height: f32,
}

impl Default for Bobbing {
    fn default() -> Self {
        Self{period: 2., height: 0.2}
    }
}

#[derive(Clone, Copy, Debug)]
struct Circle {
    radius: f32,
    height: f32,
}

impl From<(f32, f32)> for Circle {
    fn from((radius, height): (f32, f32)) -> Self {
        Self{radius, height}
    }
}

fn make_circles(
    circles: impl IntoIterator<Item=Circle>,
    resolution: u32,
    gap: f32,
) -> impl Iterator<Item=[f32; 3]> {
    return [0..resolution].into_iter()
        .cycle().zip(circles.into_iter())
        .flat_map(move |(range, circle)| {
            range.into_iter().map(move |i| {
                let theta = (i as f32)/(resolution as f32 - 1.) * (2.0*std::f32::consts::PI - gap);
                let r = circle.radius;
                let h = circle.height;
                [r*theta.cos(), r*theta.sin(), h]
            })
        })
}

pub(crate) struct PartialMesh {
    positions: Vec<[f32; 3]>,
    normals: Vec<[f32; 3]>,
    indices: Vec<u32>,
}

impl PartialMesh {
    pub(crate) fn merge_into(self, mesh: &mut Mesh) {
        let offset = mesh.attribute(Mesh::ATTRIBUTE_POSITION).map(|a| a.len());
        if let Some(offset) = offset {
            if let Some(Indices::U32(indices)) = mesh.indices_mut() {
                indices.extend(self.indices.into_iter().map(|i| i + offset as u32));
            } else {
                mesh.set_indices(Some(Indices::U32(self.indices.into_iter().map(|i| i + offset as u32).collect())));
            }

            if let Some(VertexAttributeValues::Float32x3(current_positions)) = mesh.attribute_mut(Mesh::ATTRIBUTE_POSITION) {
                current_positions.extend(self.positions.into_iter());

                if let Some(VertexAttributeValues::Float32x3(current_normals)) = mesh.attribute_mut(Mesh::ATTRIBUTE_NORMAL) {
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

fn make_boxy_wrap(
    circles: [Circle; 2],
    segments: u32,
) -> PartialMesh {
    let (bottom_circle, top_circle) = if circles[0].height < circles[1].height {
        (circles[0], circles[1])
    } else {
        (circles[1], circles[0])
    };

    let positions: Vec<[f32; 3]> = make_circles(
        [bottom_circle, bottom_circle, top_circle, top_circle],
        segments+1,
        0.,
    ).collect();

    let indices = [
        [0, 3*segments+4, 2*segments+2, 0, segments+2, 3*segments+4]
    ].into_iter().cycle().enumerate()
    .flat_map(|(i, values)| {
        values.into_iter().map(move |s| { s + i as u32 })
    }).take(6*segments as usize).collect();

    positions.len();
    let mut normals = Vec::new();
    normals.resize(positions.len(), [0., 0., 0.]);
    for i in 0..segments {
        let v0 = (i+0) as usize;
        let v1 = (i + 3*segments+4) as usize;
        let v2 = (i + 2*segments+2) as usize;
        let v3 = (i + segments+2) as usize;
        let p0: Vec3 = positions[v0].into();
        let p1: Vec3 = positions[v1].into();
        let p2: Vec3 = positions[v2].into();
        let n = (p1 - p0).cross(p2 - p0).normalize();
        [v0, v1, v2, v3].into_iter().for_each(|v| {
            normals[v] = n.into();
        });
    }

    return PartialMesh{positions, normals, indices}
}

fn make_pyramid(
    circle: Circle,
    peak: [f32; 3],
    segments: u32
) -> PartialMesh {
    let positions: Vec<[f32; 3]> = make_circles([circle, circle], segments+1, 0.)
        .chain([peak].into_iter().cycle().take(segments as usize)).collect();

    let peak_start = 2*segments+2;
    let complement_start = segments+2;
    let indices = if peak[2] < circle.height {
        [[0, peak_start, complement_start]]
    } else {
        [[0, complement_start, peak_start]]
    }.into_iter().cycle().enumerate()
    .flat_map(|(i, values)| {
        values.into_iter().map(move |s| s + i as u32)
    }).take(3*segments as usize).collect();

    let mut normals = Vec::new();
    normals.resize(positions.len(), [0., 0., 0.]);
    for i in 0..segments {
        let v0 = (i+0) as usize;
        let v1 = (i+complement_start) as usize;
        let vp = (i+peak_start) as usize;
        let p0: Vec3 = positions[v0].into();
        let p1: Vec3 = positions[v1].into();
        let p2: Vec3 = positions[vp].into();
        let n = if peak[2] < circle.height {
            (p2 - p0).cross(p1 - p0)
        } else {
            (p1 - p0).cross(p2 - p0)
        }.normalize();

        [v0, v1, vp].into_iter().for_each(|v| {
            normals[v] = n.into();
        });
    }

    return PartialMesh{positions, normals, indices};
}

fn make_dagger_mesh() -> Mesh {

    let lower_ring = Circle{radius: 0.01, height: 0.1};
    let upper_ring = Circle{radius: 0.02, height: 0.4};
    let top_height = 0.42;
    let segments = 4u32;

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    make_boxy_wrap([lower_ring, upper_ring], segments).merge_into(&mut mesh);
    make_pyramid(upper_ring, [0., 0., top_height], segments).merge_into(&mut mesh);
    make_pyramid(lower_ring, [0., 0., 0.], segments).merge_into(&mut mesh);
    return mesh;
}

fn make_halo_mesh() -> Mesh {
    let inner_ring = 1.0;
    let mid_ring = 1.1 * inner_ring;
    let outer_ring = 1.2 * inner_ring;
    let peak = 0.01;
    let segments = 100u32;
    let gap = 60_f32.to_radians();

    let positions: Vec<[f32; 3]> = make_circles(
        [(inner_ring, 0.).into(), (mid_ring, peak).into(), (outer_ring, 0.).into()],
        segments,
        gap
    ).collect();

    let colors: Vec<[f32; 4]> = [[1., 1., 1., 1.]].into_iter().cycle().take(2*segments as usize)
        .chain(
            [[1., 1., 1., 0.]].into_iter().cycle().take(segments as usize)
        ).collect();

    let normals: Vec<[f32; 3]> = [[0., 0., 1.]].into_iter().cycle().take(positions.len()).collect();

    let indices = Indices::U32([
        [0u32, segments, segments+1u32, 0u32, segments+1u32, 1u32]
    ].into_iter().cycle().enumerate()
    .flat_map(|(cycle, values)| {
        [(cycle as u32, values)].into_iter().cycle().enumerate().take(segments as usize - 1)
        .flat_map(|(segment, (cycle, values))| {
            values.map(|s| {
                cycle*segments + segment as u32 + s
            })
        })
    }).take(6*2*segments as usize)
    .chain(
        [0, 2*segments, segments, 3*segments-1, segments-1, 2*segments-1]
    )
    .collect());

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_COLOR, colors);
    mesh.set_indices(Some(indices));
    return mesh;
}

#[derive(Default)]
pub struct InteractionPlugin<T> {
    for_app_state: T,
}

impl<T> InteractionPlugin<T> {
    pub fn new(for_app_state: T ) -> Self {
        Self{for_app_state}
    }
}

impl<T: Send + Sync + Clone + Hash + Eq + Debug + 'static> Plugin for InteractionPlugin<T> {
    fn build(&self, app: &mut App) {
        app
            .add_startup_system(init_cursor)
            .add_system_set(
                SystemSet::on_update(self.for_app_state.clone())
                .with_system(update_cursor_transform.after(PickingSystem::UpdateIntersections))
                .with_system(update_spinning_animations)
                .with_system(update_bobbing_animations)
            );
    }
}

pub fn init_cursor(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    handles: Res<SiteAssets>,
) {
    let mut cursor_builder = commands.spawn_bundle(SpatialBundle::default());
    let (selection_cursor, dagger_cursor, vertex_cursor) = cursor_builder.add_children(|cursor| {
        let selection_cursor = cursor.spawn_bundle(PbrBundle{
            // mesh: meshes.add(selection_cursor_mesh()),
            transform: Transform::from_scale([0.2, 0.2, 1.].into()),
            mesh: meshes.add(make_halo_mesh()),
            material: materials.add(StandardMaterial {
                base_color: Color::WHITE,
                alpha_mode: AlphaMode::Blend,
                unlit: true,
                depth_bias: -1.0,
                ..default()
            }),
            ..default()
        }).insert(Spinning::default()).id();

        let dagger_cursor = cursor.spawn_bundle(PbrBundle{
            mesh: meshes.add(make_dagger_mesh()),
            material: materials.add(StandardMaterial{
                base_color: Color::WHITE,
                ..default()
            }),
            ..default()
        }).insert(Spinning::default())
        .insert(Bobbing::default()).id();

        let vertex_cursor = cursor.spawn_bundle(PbrBundle{
            transform: Transform{
                rotation: Quat::from_rotation_x(90_f32.to_radians()),
                ..default()
            },
            mesh: handles.vertex_mesh.clone(),
            material: materials.add(Color::rgba(0.98, 0.91, 0.28, 0.5).into()),
            visibility: Visibility{is_visible: false},
            ..default()
        }).id();

        return (selection_cursor, dagger_cursor, vertex_cursor);
    });

    cursor_builder.insert(Cursor{halo: selection_cursor, dagger: dagger_cursor, vertex: vertex_cursor});
}

pub fn update_cursor_transform(
    intersections: Query<&Intersection<PickingRaycastSet>>,
    mut cursor: Query<&mut Transform, With<Cursor>>,
) {
    for intersection in &intersections {
        if let Some(mut transform) = cursor.get_single_mut().ok() {
            if let Some(ray) = intersection.normal_ray() {
                *transform = Transform::from_matrix(ray.to_aligned_transform([0., 0., 1.].into()))
            }
        }
    }
}

pub fn update_spinning_animations(
    mut spinners: Query<(&mut Transform, &Spinning, &ComputedVisibility)>,
    now: Res<Time>,
) {
    for (mut tf, spin, visibility) in &mut spinners {
        if visibility.is_visible_in_view() {
            let angle = 2.*std::f32::consts::PI * now.seconds_since_startup() as f32 / spin.period;
            tf.as_mut().rotation = Quat::from_rotation_z(angle);
        }
    }
}

pub fn update_bobbing_animations(
    mut bobbers: Query<(&mut Transform, &Bobbing, &ComputedVisibility)>,
    now: Res<Time>,
) {
    for (mut tf, bob, visibility) in &mut bobbers {
        if visibility.is_visible_in_view() {
            let theta = 2.*std::f32::consts::PI * now.seconds_since_startup() as f32 / bob.period;
            tf.as_mut().translation[2] = bob.height*(1.-theta.cos())/2.0;
        }
    }
}
