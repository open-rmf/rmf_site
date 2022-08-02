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
    site_map::{SiteAssets, SiteMapCurrentLevel, LanePieces},
    traffic_editor::EditableTag,
    lane::{Lane, PASSIVE_LANE_HEIGHT, SELECTED_LANE_HEIGHT, HOVERED_LANE_HEIGHT},
    spawner::VerticesManagers,
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
use std::{fmt::Debug, hash::Hash, collections::HashSet};

#[derive(Clone, Debug)]
pub struct InteractionAssets {
    pub dagger_mesh: Handle<Mesh>,
    pub dagger_material: Handle<StandardMaterial>,
    pub halo_mesh: Handle<Mesh>,
    pub halo_material: Handle<StandardMaterial>,
}

impl FromWorld for InteractionAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let dagger_mesh = meshes.add(make_dagger_mesh());
        let halo_mesh = meshes.add(make_halo_mesh());

        let mut materials = world.get_resource_mut::<Assets<StandardMaterial>>().unwrap();
        let halo_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            depth_bias: -1.0,
            ..default()
        });
        let dagger_material = materials.add(StandardMaterial{
            base_color: Color::WHITE,
            ..default()
        });

        Self{
            dagger_mesh,
            dagger_material,
            halo_mesh,
            halo_material,
        }
    }
}

#[derive(Debug, Component)]
pub struct Cursor {
    halo: Entity,
    dagger: Entity,
    vertex: Entity,
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
    heights: (f32, f32),
}

impl Bobbing {
    pub fn between(h_min: f32, h_max: f32) -> Self {
        Self{heights: (h_min, h_max), ..default()}
    }
}

impl Default for Bobbing {
    fn default() -> Self {
        Self{period: 2., heights: (0., 0.2)}
    }
}

impl From<(f32, f32)> for Bobbing {
    fn from(heights: (f32, f32)) -> Self {
        Self{heights, ..default()}
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
    }).take(6*2*(segments as usize - 1))
    .chain(
        [0, 2*segments, segments, 3*segments-1, segments-1, 2*segments-1]
    )
    .collect());

    if let Indices::U32(indices) = &indices {
        dbg!(positions.len(), normals.len());
        dbg!(indices.iter().max());
        dbg!(indices.len());
        for (i, index) in indices.iter().enumerate() {
            if *index >= 300 {
                println!("{i} => {index}");
            }
        }
    }

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
            .init_resource::<InteractionAssets>()
            .add_startup_system(init_cursor)
            .add_system_set(
                SystemSet::on_update(self.for_app_state.clone())
                .with_system(update_cursor_transform.after(PickingSystem::UpdateIntersections))
                .with_system(update_spinning_animations)
                .with_system(update_bobbing_animations)
                .with_system(update_vertex_visual_cues)
                .with_system(update_lane_visual_cues)
                .with_system(update_floor_and_wall_visual_cues)
            );
    }
}

pub fn init_cursor(
    mut commands: Commands,
    mut materials: ResMut<Assets<StandardMaterial>>,
    site_assets: Res<SiteAssets>,
    interaction_assets: Res<InteractionAssets>,
) {
    let mut cursor_builder = commands.spawn_bundle(SpatialBundle::default());
    let (selection_cursor, dagger_cursor, vertex_cursor) = cursor_builder.add_children(|cursor| {
        let selection_cursor = cursor.spawn_bundle(PbrBundle{
            transform: Transform::from_scale([0.2, 0.2, 1.].into()),
            mesh: interaction_assets.halo_mesh.clone(),
            material: interaction_assets.halo_material.clone(),
            ..default()
        }).insert(Spinning::default()).id();

        let dagger_cursor = cursor.spawn_bundle(PbrBundle{
            mesh: interaction_assets.dagger_mesh.clone(),
            material: interaction_assets.dagger_material.clone(),
            ..default()
        }).insert(Spinning::default())
        .insert(Bobbing::default()).id();

        let vertex_cursor = cursor.spawn_bundle(PbrBundle{
            transform: Transform{
                rotation: Quat::from_rotation_x(90_f32.to_radians()),
                ..default()
            },
            mesh: site_assets.vertex_mesh.clone(),
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
            let dh = bob.heights.1 - bob.heights.0;
            tf.as_mut().translation[2] = dh*(1.-theta.cos())/2.0 + bob.heights.0;
        }
    }
}

pub fn set_visibility(
    entity: Entity,
    q_visibility: &mut Query<&mut Visibility>,
    visible: bool,
) {
    if let Some(mut visibility) = q_visibility.get_mut(entity).ok() {
        visibility.is_visible = visible;
    }
}

fn recursive_set_material(
    parent: Entity,
    to_material: &Handle<StandardMaterial>,
    q_material: &mut Query<&mut Handle<StandardMaterial>>,
    q_children: &Query<&Children>,
    q_tags: &Query<&EditableTag>,
) {
    if let Some(mut material) = q_material.get_mut(parent).ok() {
        *material = to_material.clone();
    }

    if let Some(children) = q_children.get(parent).ok() {
        for child in children {
            if q_tags.get(*child).ok().filter(|t| !t.ignore()).is_some() {
                recursive_set_material(*child, to_material, q_material, q_children, q_tags);
            }
        }
    }
}

fn set_height(
    entity: Entity,
    spatial: &mut Query<&mut Transform>,
    height: f32,
) {
    if let Some(mut tf) = spatial.get_mut(entity).ok() {
        tf.as_mut().translation[2] = height;
    }
}

fn set_material(
    entity: Entity,
    to_material: &Handle<StandardMaterial>,
    q_materials: &mut Query<&mut Handle<StandardMaterial>>,
) {
    if let Some(mut m) = q_materials.get_mut(entity).ok() {
        *m = to_material.clone();
    }
}

fn set_bobbing(
    entity: Entity,
    min_height: f32,
    max_height: f32,
    q_bobbing: &mut Query<&mut Bobbing>,
) {
    if let Some(mut b) = q_bobbing.get_mut(entity).ok() {
        b.heights = (min_height, max_height);
    }
}

#[derive(Component, Debug, PartialEq, Eq)]
pub struct Hovering {
    /// The cursor is hovering on this object specifically
    pub is_hovering: bool,
    /// The cursor is hovering on a different object which wants this vertex
    /// to be highlighted.
    pub support_hovering: HashSet<Entity>,
}

impl Hovering {
    pub fn cue(&self) -> bool {
        self.is_hovering || !self.support_hovering.is_empty()
    }
}

impl Default for Hovering {
    fn default() -> Self {
        Self{is_hovering: false, support_hovering: Default::default()}
    }
}

#[derive(Component, Debug, PartialEq, Eq)]
pub struct Selected {
    /// This object has been selected
    pub is_selected: bool,
    /// Another object is selected but wants this vertex to be highlighted
    pub support_selected: HashSet<Entity>,
}

impl Selected {
    pub fn cue(&self) -> bool {
        self.is_selected || !self.support_selected.is_empty()
    }
}

impl Default for Selected {
    fn default() -> Self {
        Self{is_selected: false, support_selected: Default::default()}
    }
}

#[derive(Component)]
pub struct VertexVisualCue {
    pub dagger: Entity,
    pub halo: Entity,
    pub body: Entity,
}

pub fn update_vertex_visual_cues(
    vertices: Query<(Entity, &Hovering, &Selected, &VertexVisualCue)>,
    hover_changes: Query<ChangeTrackers<Hovering>>,
    select_changes: Query<ChangeTrackers<Selected>>,
    mut bobbing: Query<&mut Bobbing>,
    mut visibility: Query<&mut Visibility>,
    mut materials: Query<&mut Handle<StandardMaterial>>,
    cursor: Query<Entity, With<Cursor>>,
    site_assets: Res<SiteAssets>,
) {
    for (v, hovering, selected, cue) in &vertices {
        let hover_changed = hover_changes.get(v).ok().filter(|h| h.is_changed()).is_some();
        let select_changed = select_changes.get(v).ok().filter(|s| s.is_changed()).is_some();
        if hover_changed || select_changed {
            // dbg!(v, hovering, selected);
            if hovering.cue() || selected.cue() {
                set_visibility(cue.dagger, &mut visibility, true);
                set_visibility(cue.halo, &mut visibility, true);
            }

            if hovering.cue() {
                set_visibility(cursor.single(), &mut visibility, false);
            }

            let vertex_height = 0.15 + 0.05/2.;
            if selected.cue() {
                set_bobbing(cue.dagger, vertex_height, vertex_height, &mut bobbing);
            }

            if hovering.cue() && selected.cue() {
                set_material(cue.body, &site_assets.hover_select_vertex_material, &mut materials);
            } else if hovering.cue() {
                set_material(cue.body, &site_assets.vertex_material, &mut materials);
                set_bobbing(cue.dagger, vertex_height, vertex_height+0.2, &mut bobbing);
            } else if !hovering.cue() && !selected.cue() {
                set_material(cue.body, &site_assets.vertex_material, &mut materials);
                set_visibility(cue.dagger, &mut visibility, false);
                set_visibility(cue.halo, &mut visibility, false);
            }
        }
    }
}

#[derive(Component, Default)]
pub struct LaneVisualCue {
    /// If the lane is using support from some vertices, the entities of those
    /// vertices will be noted here
    supporters: Option<(Entity, Entity)>,
}

pub fn update_lane_visual_cues(
    mut lanes: Query<(Entity, &Hovering, &Selected, &Lane, &LanePieces, &mut LaneVisualCue, &mut Transform), (Without<VertexVisualCue>, ChangeTrackers<Hovering>, ChangeTrackers<Selected>, ChangeTrackers<Lane>)>,
    mut vertices: Query<(&mut Hovering, &mut Selected), With<VertexVisualCue>>,
    mut materials: Query<&mut Handle<StandardMaterial>>,
    site_assets: Res<SiteAssets>,
    vm: Res<VerticesManagers>,
    level: Res<Option<SiteMapCurrentLevel>>,
) {
    let level = match level.as_ref() {
        Some(level) => level,
        None => { return; }
    };
    for (l, hovering, selected, lane, pieces, mut cue, mut tf) in &mut lanes {
        // println!("Change in lane {l:?}");
        if let Some(vm) = vm.0.get(&level.0) {
            if let (Some(v0), Some(v1)) = (vm.id_to_entity(lane.0), vm.id_to_entity(lane.1)) {
                if let Some((old_v0, old_v1)) = cue.supporters {
                    // If we have supporters that are out of date, clear them out.
                    // This can happen if a user changes the start or end vertices
                    // of the lane.
                    if (old_v0, old_v1) != (v0, v1) {
                        for v in [old_v0, old_v1] {
                            if let Some((mut hover, mut selected)) = vertices.get_mut(v).ok() {
                                hover.support_hovering.remove(&l);
                                selected.support_selected.remove(&l);
                            }
                        }
                    }
                }

                if hovering.cue() || selected.cue() {
                    cue.supporters = Some((v0, v1));
                } else {
                    cue.supporters = None;
                }

                if let Some([(mut hover_v0, mut selected_v0), (mut hover_v1, mut selected_v1)]) = vertices.get_many_mut([v0, v1]).ok() {
                    if hovering.cue() {
                        hover_v0.support_hovering.insert(l);
                        hover_v1.support_hovering.insert(l);
                    } else {
                        hover_v0.support_hovering.remove(&l);
                        hover_v1.support_hovering.remove(&l);
                    }

                    if selected.cue() {
                        selected_v0.support_selected.insert(l);
                        selected_v1.support_selected.insert(l);
                    } else {
                        selected_v0.support_selected.remove(&l);
                        selected_v1.support_selected.remove(&l);
                    }
                }

                let (m, h) = if hovering.cue() && selected.cue() {
                    (&site_assets.hover_select_lane_material, HOVERED_LANE_HEIGHT)
                } else if hovering.cue() {
                    (&site_assets.hover_lane_material, HOVERED_LANE_HEIGHT)
                } else if selected.cue() {
                    (&site_assets.select_lane_material, SELECTED_LANE_HEIGHT)
                } else {
                    (&site_assets.passive_lane_material, PASSIVE_LANE_HEIGHT)
                };

                for e in pieces.segments {
                    set_material(e, m, &mut materials);
                }

                tf.translation.z = h;
            }
        }
    }
}

#[derive(Component)]
pub struct FloorVisualCue;

#[derive(Component)]
pub struct WallVisualCue;

pub fn update_floor_and_wall_visual_cues(
    floors: Query<&Hovering, With<FloorVisualCue>>,
    walls: Query<&Hovering, With<WallVisualCue>>,
    cursor: Query<Entity, With<Cursor>>,
    mut visibility: Query<&mut Visibility>,
) {
    for hovering in floors.iter().chain(walls.iter()) {
        if hovering.cue() {
            if let Some(mut v) = visibility.get_mut(cursor.single()).ok() {
                v.is_visible = true;
            }
        }
    }
}
