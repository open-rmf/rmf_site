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
    lane::{Lane, HOVERED_LANE_HEIGHT, PASSIVE_LANE_HEIGHT, SELECTED_LANE_HEIGHT},
    site_map::{LanePieces, SiteAssets, SiteMapCurrentLevel},
    spawner::VerticesManagers,
    traffic_editor::{ElementDeleted, EditableTag},
    camera_controls::CameraControls,
};
use bevy::{
    prelude::*,
    render::mesh::{Indices, Mesh, PrimitiveTopology, VertexAttributeValues},
    math::Affine3A,
};
use bevy_mod_picking::{PickingRaycastSet, PickingSystem, PickableBundle};
use bevy_mod_raycast::{Intersection, Ray3d};
use std::{collections::HashSet, fmt::Debug, hash::Hash};

#[derive(Clone, Debug)]
pub struct InteractionAssets {
    pub dagger_mesh: Handle<Mesh>,
    pub dagger_material: Handle<StandardMaterial>,
    pub halo_mesh: Handle<Mesh>,
    pub halo_material: Handle<StandardMaterial>,
    pub arrow_mesh: Handle<Mesh>,
    pub flat_square_mesh: Handle<Mesh>,
    pub x_axis_materials: DraggableMaterialSet,
    pub y_axis_materials: DraggableMaterialSet,
    pub z_plane_materials: DraggableMaterialSet,
}

#[derive(Component)]
pub struct Selectable {
    /// Toggle whether this entity is selectable
    pub is_selectable: bool,
    /// What element of the site is being selected when this entity is clicked
    pub element: Entity,
}

impl Selectable {
    fn new(element: Entity) -> Self {
        Selectable{is_selectable: true, element}
    }
}

impl InteractionAssets {

    pub fn make_draggable_axis(
        &self,
        command: &mut Commands,
        // What entity will be moved when this gizmo is dragged
        for_entity: Entity,
        // What entity should be the parent frame of this gizmo
        parent: Entity,
        material_set: DraggableMaterialSet,
        offset: Vec3,
        rotation: Quat,
        scale: f32,
    ) -> Entity {
        return command.entity(parent).add_children(|parent| {
            let id = parent.spawn_bundle(PbrBundle{
                transform: Transform::from_rotation(
                    rotation
                ).with_translation(offset)
                .with_scale(Vec3::splat(scale)),
                mesh: self.arrow_mesh.clone(),
                material: material_set.passive.clone(),
                ..default()
            })
            .insert(DragAxis{
                along: [0., 0., 1.].into(),
            })
            .insert(Draggable::new(for_entity, material_set))
            .insert(EditableTag::Ignore).id();
            id
        });
    }

    pub fn make_vertex_draggable(
        &self,
        command: &mut Commands,
        vertex: Entity,
        cue: &mut VertexVisualCue,
    ) {
        let drag_parent = command.entity(vertex).add_children(|parent| {
            parent.spawn_bundle(SpatialBundle::default()).id()
        });

        let height = 0.01;
        let scale = 0.2;
        let offset = 0.15;
        for (m, p, r) in [
            (
                self.x_axis_materials.clone(),
                Vec3::new(offset, 0., height),
                Quat::from_rotation_y(90_f32.to_radians()),
            ),
            (
                self.x_axis_materials.clone(),
                Vec3::new(-offset, 0., height),
                Quat::from_rotation_y(-90_f32.to_radians()),
            ),
            (
                self.y_axis_materials.clone(),
                Vec3::new(0., offset, height),
                Quat::from_rotation_x(-90_f32.to_radians()),
            ),
            (
                self.y_axis_materials.clone(),
                Vec3::new(0., -offset, height),
                Quat::from_rotation_x(90_f32.to_radians()),
            )
        ] {
            self.make_draggable_axis(command, vertex, drag_parent, m, p, r, scale);
        }

        command.entity(drag_parent).add_children(|parent| {
            parent.spawn_bundle(PbrBundle{
                transform: Transform::from_translation([0., 0., height].into())
                .with_scale(Vec3::splat(0.75*(scale+offset))),
                mesh: self.flat_square_mesh.clone(),
                material: self.z_plane_materials.passive.clone(),
                ..default()
            })
            .insert(DragPlane{
                in_plane: Vec3::new(0., 0., 1.),
            })
            .insert(Draggable::new(vertex, self.z_plane_materials.clone()))
            .insert(EditableTag::Ignore);
        });

        cue.drag = Some(drag_parent);
    }
}

impl FromWorld for InteractionAssets {
    fn from_world(world: &mut World) -> Self {
        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let dagger_mesh = meshes.add(make_dagger_mesh());
        let halo_mesh = meshes.add(make_halo_mesh());
        let arrow_mesh = meshes.add(make_arrow_mesh());
        let flat_square_mesh = meshes.add(make_flat_square_mesh());

        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let halo_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            alpha_mode: AlphaMode::Blend,
            unlit: true,
            ..default()
        });
        let dagger_material = materials.add(StandardMaterial {
            base_color: Color::WHITE,
            ..default()
        });
        let x_axis_materials = DraggableMaterialSet::make_x_axis(&mut materials);
        let y_axis_materials = DraggableMaterialSet::make_y_axis(&mut materials);
        let z_plane_materials = DraggableMaterialSet::make_z_plane(&mut materials);

        Self {
            dagger_mesh,
            dagger_material,
            halo_mesh,
            halo_material,
            arrow_mesh,
            flat_square_mesh,
            x_axis_materials,
            y_axis_materials,
            z_plane_materials,
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
        Self { period: 2. }
    }
}

#[derive(Debug, Component)]
pub struct Bobbing {
    period: f32,
    heights: (f32, f32),
}

impl Bobbing {
    pub fn between(h_min: f32, h_max: f32) -> Self {
        Self {
            heights: (h_min, h_max),
            ..default()
        }
    }
}

impl Default for Bobbing {
    fn default() -> Self {
        Self {
            period: 2.,
            heights: (0., 0.2),
        }
    }
}

impl From<(f32, f32)> for Bobbing {
    fn from(heights: (f32, f32)) -> Self {
        Self {
            heights,
            ..default()
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct InitialDragConditions {
    click_point: Vec3,
    entity_tf: Transform,
}

#[derive(Debug, Clone)]
pub struct DraggableMaterialSet {
    passive: Handle<StandardMaterial>,
    hover: Handle<StandardMaterial>,
    drag: Handle<StandardMaterial>,
}

impl DraggableMaterialSet {
    pub fn make_x_axis(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self{
            passive: materials.add(Color::rgb(1., 0., 0.).into()),
            hover: materials.add(Color::rgb(1.0, 0.3, 0.3).into()),
            drag: materials.add(Color::rgb(0.7, 0., 0.).into()),
        }
    }

    pub fn make_y_axis(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self{
            passive: materials.add(Color::rgb(0., 0.9, 0.).into()),
            hover: materials.add(Color::rgb(0.5, 1.0, 0.5).into()),
            drag: materials.add(Color::rgb(0., 0.6, 0.).into()),
        }
    }

    pub fn make_z_plane(materials: &mut Mut<Assets<StandardMaterial>>) -> Self {
        Self{
            passive: materials.add(Color::rgba(0., 0., 1., 0.6).into()),
            hover: materials.add(Color::rgba(0.3, 0.3, 1., 0.6).into()),
            drag: materials.add(Color::rgba(0., 0., 0.7, 0.9).into()),
        }
    }
}

#[derive(Component, Debug, Clone)]
pub struct Draggable {
    for_entity: Entity,
    materials: DraggableMaterialSet,
    initial: Option<InitialDragConditions>,
}

impl Draggable {
    pub fn new(
        for_entity: Entity,
        materials: DraggableMaterialSet,
    ) -> Self {
        Self{for_entity, materials, initial: None}
    }
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DragAxis {
    /// The gizmo can only be dragged along this axis
    along: Vec3,
}

#[derive(Component, Debug, Clone, Copy)]
pub struct DragPlane {
    /// The gizmo can only be dragged in the plane orthogonal to this vector
    in_plane: Vec3,
}

#[derive(Clone, Copy, Debug)]
struct Circle {
    radius: f32,
    height: f32,
}

impl From<(f32, f32)> for Circle {
    fn from((radius, height): (f32, f32)) -> Self {
        Self { radius, height }
    }
}

fn make_circles(
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
                    (i as f32) / (resolution as f32 - 1.) * (2.0 * std::f32::consts::PI - gap);
                let r = circle.radius;
                let h = circle.height;
                [r * theta.cos(), r * theta.sin(), h]
            })
        });
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

fn make_boxy_wrap(circles: [Circle; 2], segments: u32) -> PartialMesh {
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

    return PartialMesh {
        positions,
        normals,
        indices,
    };
}

fn make_smooth_wrap(circles: [Circle; 2], resolution: u32) -> PartialMesh {
    let (bottom_circle, top_circle) = if circles[0].height < circles[1].height {
        (circles[0], circles[1])
    } else {
        (circles[1], circles[0])
    };

    let positions: Vec<[f32; 3]> = make_circles(
        [bottom_circle, top_circle], resolution, 0.
    )
    .collect();

    let top_start = resolution;
    let indices = [[0, 1, top_start+1, 0, top_start+1, top_start]]
        .into_iter()
        .cycle()
        .enumerate()
        .flat_map(|(i, values)| values.into_iter().map(move |s| s + i as u32))
        .take(6*(resolution-1) as usize)
        .collect();

    let mut normals = Vec::new();
    normals.resize(positions.len(), [0., 0., 1.]);
    for i in 0..resolution {
        let theta = (i as f32)/(resolution as f32 - 1.) * 2.*std::f32::consts::PI;
        let dr = top_circle.radius - bottom_circle.radius;
        let dh = top_circle.height - bottom_circle.height;
        let phi = dr.atan2(dh);
        let r_y = Affine3A::from_rotation_y(phi);
        let r_z = Affine3A::from_rotation_z(theta);
        let n = (r_z*r_y).transform_vector3([1., 0., 0.,].into());
        normals[i as usize] = n.into();
        normals[(i+top_start) as usize] = n.into();
    }

    return PartialMesh {
        positions,
        normals,
        indices,
    }
}

fn make_pyramid(circle: Circle, peak: [f32; 3], segments: u32) -> PartialMesh {
    let positions: Vec<[f32; 3]> = make_circles([circle, circle], segments + 1, 0.)
        .chain([peak].into_iter().cycle().take(segments as usize))
        .collect();

    let peak_start = 2 * segments + 2;
    let complement_start = segments + 2;
    let indices = if peak[2] < circle.height {
        [[0, peak_start, complement_start]]
    } else {
        [[0, complement_start, peak_start]]
    }
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

    return PartialMesh {
        positions,
        normals,
        indices,
    };
}

fn make_cone(circle: Circle, peak: [f32; 3], resolution: u32) -> PartialMesh {
    let positions: Vec<[f32; 3]> = make_circles([circle], resolution+1, 0.)
        .take(resolution as usize) // skip the last vertex which would close the circle
        .chain([peak].into_iter().cycle().take(resolution as usize))
        .collect();

    let peak_start = resolution;
    let indices: Vec<u32> = [[0, 1, peak_start]]
        .into_iter()
        .cycle()
        .enumerate()
        .flat_map(|(i, values)| values.into_iter().map(move |s| s + i as u32))
        .take(3*(resolution as usize - 1))
        .chain([peak_start-1, 0, (positions.len()-1) as u32])
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

        let theta = (i as f32)/(resolution as f32) * 2.0 * std::f32::consts::PI;
        normals[i as usize] = calculate_normal(theta);

        let mid_theta = (i as f32 + 0.5)/(resolution as f32) * 2.0 * std::f32::consts::PI;
        normals[(i + peak_start) as usize] = calculate_normal(mid_theta);
    }

    return PartialMesh{positions, normals, indices};
}

fn make_bottom_circle(circle: Circle, resolution: u32) -> PartialMesh {
    let positions: Vec<[f32; 3]> = make_circles([circle], resolution, 0.)
        .take(resolution as usize - 1) // skip the vertex which would close the circle
        .chain([[0., 0., circle.height]].into_iter())
        .collect();

    let peak = positions.len() as u32 - 1;
    let indices: Vec<u32> = (0..resolution-1)
        .into_iter()
        .flat_map(|i| [i, peak, i+1].into_iter())
        .chain([resolution-1, peak, 0])
        .collect();

    let normals: Vec<[f32; 3]> = [[0., 0., -1.]]
        .into_iter()
        .cycle()
        .take(positions.len())
        .collect();

    return PartialMesh{positions, normals, indices};
}

fn make_dagger_mesh() -> Mesh {
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
    make_pyramid(lower_ring, [0., 0., 0.], segments).merge_into(&mut mesh);
    return mesh;
}

fn make_arrow_mesh() -> Mesh {
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

fn make_flat_square_mesh() -> Mesh {
    let positions: Vec<[f32; 3]> = [
        [-1., -1., 0.],
        [1., -1., 0.],
        [1., 1., 0.],
        [-1., 1., 0.],
    ].into_iter().cycle().take(8).collect();

    let indices = Indices::U32(
        [
            0, 1, 2, 0, 2, 3,
            4, 6, 5, 4, 7, 6,
        ].into_iter().collect()
    );

    let normals: Vec<[f32; 3]> = [
        [0., 0., 1.]
    ].into_iter().cycle().take(4)
    .chain([
        [0., 0., -1.]
    ].into_iter().cycle().take(4)).collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.set_indices(Some(indices));
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

#[derive(Default)]
pub struct InteractionPlugin<T> {
    for_app_state: T,
}

impl<T> InteractionPlugin<T> {
    pub fn new(for_app_state: T) -> Self {
        Self { for_app_state }
    }
}

impl<T: Send + Sync + Clone + Hash + Eq + Debug + 'static> Plugin for InteractionPlugin<T> {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<InteractionAssets>()
            .init_resource::<Dragging>()
            .add_event::<ElementDeleted>()
            .add_startup_system(init_cursor)
            .add_system_set(
                SystemSet::on_update(self.for_app_state.clone())
                    .with_system(update_cursor_transform.after(PickingSystem::UpdateIntersections))
                    .with_system(update_spinning_animations)
                    .with_system(update_bobbing_animations)
                    .with_system(update_vertex_visual_cues)
                    .with_system(update_lane_visual_cues)
                    .with_system(update_floor_and_wall_visual_cues)
                    .with_system(remove_deleted_supports_from_interactions)
                    .with_system(make_gizmos_pickable)
                    .with_system(update_drag_click_start)
                    .with_system(update_drag_release)
                    .with_system(
                        update_drag_motions
                        .after(update_drag_click_start)
                        .after(update_drag_release)
                    ),
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
        let selection_cursor = cursor
            .spawn_bundle(PbrBundle {
                transform: Transform::from_scale([0.2, 0.2, 1.].into()),
                mesh: interaction_assets.halo_mesh.clone(),
                material: interaction_assets.halo_material.clone(),
                ..default()
            })
            .insert(Spinning::default())
            .id();

        let dagger_cursor = cursor
            .spawn_bundle(PbrBundle {
                mesh: interaction_assets.dagger_mesh.clone(),
                material: interaction_assets.dagger_material.clone(),
                ..default()
            })
            .insert(Spinning::default())
            .insert(Bobbing::default())
            .id();

        let vertex_cursor = cursor
            .spawn_bundle(PbrBundle {
                transform: Transform {
                    rotation: Quat::from_rotation_x(90_f32.to_radians()),
                    ..default()
                },
                mesh: site_assets.vertex_mesh.clone(),
                material: materials.add(StandardMaterial {
                    base_color: Color::rgba(0.98, 0.91, 0.28, 0.5),
                    alpha_mode: AlphaMode::Blend,
                    depth_bias: 1.0,
                    ..default()
                }),
                visibility: Visibility { is_visible: false },
                ..default()
            })
            .id();

        return (selection_cursor, dagger_cursor, vertex_cursor);
    });

    cursor_builder.insert(Cursor {
        halo: selection_cursor,
        dagger: dagger_cursor,
        vertex: vertex_cursor,
    });
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

pub fn make_gizmos_pickable(
    mut command: Commands,
    drag_axis: Query<Entity, Added<DragAxis>>,
    drag_plane: Query<Entity, Added<DragPlane>>,
) {
    for e in drag_axis.iter().chain(drag_plane.iter()) {
        command.entity(e).insert_bundle(PickableBundle::default());
    }
}

pub fn update_drag_click_start(
    mut draggables: Query<(&mut Draggable, &Interaction, &mut Handle<StandardMaterial>), Changed<Interaction>>,
    mut dragging: ResMut<Dragging>,
    mut visibility: Query<&mut Visibility>,
    cursor: Query<Entity, With<Cursor>>,
    transforms: Query<&GlobalTransform>,
    intersections: Query<&Intersection<PickingRaycastSet>>,
) {
    for (mut drag, interaction, mut material) in &mut draggables {
        match *interaction {
            Interaction::Clicked => {
                if let Some(intersection) = intersections.get_single().ok().and_then(|i| i.position()) {
                    if let Some(tf) = transforms.get(drag.for_entity).ok() {
                        dragging.is_dragging = true;
                        drag.initial = Some(InitialDragConditions{
                            click_point: intersection.clone(),
                            entity_tf: tf.compute_transform(),
                        });
                        *material = drag.materials.drag.clone();
                    }
                }
            },
            Interaction::Hovered => {
                if drag.initial.is_none() {
                    set_visibility(cursor.single(), &mut visibility, false);
                    *material = drag.materials.hover.clone();
                }
            },
            Interaction::None => {
                if drag.initial.is_none() {
                    *material = drag.materials.passive.clone();
                }
            }
        }
    }
}

pub fn update_drag_release(
    mut draggables: Query<(&mut Draggable, &mut Handle<StandardMaterial>)>,
    mut dragging: ResMut<Dragging>,
    mouse_button_input: Res<Input<MouseButton>>,
) {
    if mouse_button_input.just_released(MouseButton::Left) {
        for (mut draggable, mut material) in &mut draggables {
            if draggable.initial.is_some() {
                draggable.initial = None;
                *material = draggable.materials.passive.clone();
            }
        }

        dragging.is_dragging = false;
    }
}

pub fn update_drag_motions(
    drag_axis: Query<(&DragAxis, &Draggable, &GlobalTransform), Without<DragPlane>>,
    drag_plane: Query<(&DragPlane, &Draggable, &GlobalTransform), Without<DragAxis>>,
    mut transforms: Query<(&mut Transform, &GlobalTransform), Without<Draggable>>,
    cameras: Query<&Camera>,
    camera_controls: Query<&CameraControls>,
    mut cursor_motion: EventReader<CursorMoved>,
) {
    let cursor_position = match cursor_motion.iter().last() {
        Some(m) => m.position,
        None => { return; }
    };

    let active_camera = camera_controls.single().active_camera();
    let ray = if let Some(camera) = cameras.get(active_camera).ok() {
        let camera_tf = match transforms.get(active_camera).ok() {
            Some(tf) => tf.1.clone(),
            None => { return; }
        };

        match Ray3d::from_screenspace(cursor_position, camera, &camera_tf) {
            Some(ray) => ray,
            None => { return; }
        }
    } else {
        return;
    };

    for (axis, draggable, drag_tf) in &drag_axis {
        if let Some(initial) = &draggable.initial {
            if let Some((mut for_local_tf, for_global_tf)) = transforms.get_mut(draggable.for_entity).ok() {
                let n = drag_tf.affine().transform_vector3(axis.along).normalize_or_zero();
                let dp = ray.origin() - initial.click_point;
                let a = ray.direction().dot(n);
                let b = ray.direction().dot(dp);
                let c = n.dot(dp);

                let denom = a.powi(2) - 1.;
                if denom.abs() < 1e-3 {
                    // The rays are nearly parallel, so we should not attempt moving
                    // because the motion will be too extreme
                    continue;
                }

                let t = (a*b - c)/denom;
                let delta = t*n;
                let tf_goal = initial.entity_tf.with_translation(initial.entity_tf.translation + delta);
                let tf_parent_inv = for_local_tf.compute_affine() * for_global_tf.affine().inverse();
                *for_local_tf = Transform::from_matrix((tf_parent_inv * tf_goal.compute_affine()).into());
            }
        }
    }

    for (plane, draggable, drag_tf) in &drag_plane {
        if let Some(initial) = &draggable.initial {
            if let Some((mut for_local_tf, for_global_tf)) = transforms.get_mut(draggable.for_entity).ok() {
                let n_p = drag_tf.affine().transform_vector3(plane.in_plane).normalize_or_zero();
                let n_r = ray.direction();
                let denom = n_p.dot(n_r);
                if denom.abs() < 1e-3 {
                    // The rays are nearly parallel so we should not attempt moving
                    // because the motion will be too extreme
                    continue;
                }

                let t = (initial.click_point - ray.origin()).dot(n_p)/denom;
                let delta = ray.position(t) - initial.click_point;
                let tf_goal = initial.entity_tf.with_translation(initial.entity_tf.translation + delta);
                let tf_parent_inv = for_local_tf.compute_affine() * for_global_tf.affine().inverse();
                *for_local_tf = Transform::from_matrix((tf_parent_inv * tf_goal.compute_affine()).into());
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
            let angle =
                2. * std::f32::consts::PI * now.seconds_since_startup() as f32 / spin.period;
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
            let theta = 2. * std::f32::consts::PI * now.seconds_since_startup() as f32 / bob.period;
            let dh = bob.heights.1 - bob.heights.0;
            tf.as_mut().translation[2] = dh * (1. - theta.cos()) / 2.0 + bob.heights.0;
        }
    }
}

pub fn set_visibility(entity: Entity, q_visibility: &mut Query<&mut Visibility>, visible: bool) {
    if let Some(mut visibility) = q_visibility.get_mut(entity).ok() {
        visibility.is_visible = visible;
    }
}

// // I stopped using this function, but we might want it back if we decide that
// // we want to highlight selected/hovered models
// fn recursive_set_material(
//     parent: Entity,
//     to_material: &Handle<StandardMaterial>,
//     q_material: &mut Query<&mut Handle<StandardMaterial>>,
//     q_children: &Query<&Children>,
//     q_tags: &Query<&EditableTag>,
// ) {
//     if let Some(mut material) = q_material.get_mut(parent).ok() {
//         *material = to_material.clone();
//     }

//     if let Some(children) = q_children.get(parent).ok() {
//         for child in children {
//             if q_tags.get(*child).ok().filter(|t| !t.ignore()).is_some() {
//                 recursive_set_material(*child, to_material, q_material, q_children, q_tags);
//             }
//         }
//     }
// }

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
        Self {
            is_hovering: false,
            support_hovering: Default::default(),
        }
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
        Self {
            is_selected: false,
            support_selected: Default::default(),
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub struct Dragging {
    pub is_dragging: bool,
}

impl Default for Dragging {
    fn default() -> Self {
        Self{is_dragging: false}
    }
}

#[derive(Component)]
pub struct VertexVisualCue {
    pub dagger: Entity,
    pub halo: Entity,
    pub body: Entity,
    pub drag: Option<Entity>,
}

pub fn update_vertex_visual_cues(
    mut command: Commands,
    mut vertices: Query<(Entity, &Hovering, &Selected, &mut VertexVisualCue, ChangeTrackers<Selected>), Or<(Changed<Hovering>, Changed<Selected>)>>,
    mut bobbing: Query<&mut Bobbing>,
    mut visibility: Query<&mut Visibility>,
    mut materials: Query<&mut Handle<StandardMaterial>>,
    cursor: Query<Entity, With<Cursor>>,
    site_assets: Res<SiteAssets>,
    interaction_assets: Res<InteractionAssets>,
) {
    for (v, hovering, selected, mut cue, select_tracker) in &mut vertices {
        if hovering.cue() || selected.cue() {
            set_visibility(cue.dagger, &mut visibility, true);
        }

        if hovering.is_hovering {
            set_visibility(cursor.single(), &mut visibility, false);
        }

        if selected.cue() {
            set_visibility(cue.halo, &mut visibility, false);
        }

        let vertex_height = 0.15 + 0.05 / 2.;
        if selected.cue() {
            set_bobbing(cue.dagger, vertex_height, vertex_height, &mut bobbing);
        }

        if hovering.cue() && selected.cue() {
            set_material(cue.body, &site_assets.hover_select_material, &mut materials);
        } else if hovering.cue() {
            // Hovering but not selected
            set_visibility(cue.halo, &mut visibility, true);
            set_material(cue.body, &site_assets.hover_material, &mut materials);
            set_bobbing(cue.dagger, vertex_height, vertex_height + 0.2, &mut bobbing);
        } else if selected.cue() {
            // Selected but not hovering
            set_material(cue.body, &site_assets.select_material, &mut materials);
        } else {
            set_material(
                cue.body,
                &site_assets.passive_vertex_material,
                &mut materials,
            );
            set_visibility(cue.dagger, &mut visibility, false);
            set_visibility(cue.halo, &mut visibility, false);
        }

        if select_tracker.is_changed() {
            if selected.cue() {
                if cue.drag.is_none() {
                    interaction_assets.make_vertex_draggable(&mut command, v, cue.as_mut());
                }
            } else {
                if let Some(drag) = cue.drag {
                    command.entity(drag).despawn_recursive();
                }
                cue.drag = None;
            }
        }
    }
}

// NOTE(MXG): Currently only vertices ever have support cues, so we filter down
// to entities with VertexVisualCues. We will need to broaden that if any other
// visual cue types ever have a supporting role.
pub fn remove_deleted_supports_from_interactions(
    mut hover: Query<&mut Hovering, With<VertexVisualCue>>,
    mut select: Query<&mut Selected, With<VertexVisualCue>>,
    mut deleted_elements: EventReader<ElementDeleted>,
) {
    for deletion in deleted_elements.iter() {
        for mut h in &mut hover {
            h.support_hovering.remove(&deletion.0);
        }

        for mut s in &mut select {
            s.support_selected.remove(&deletion.0);
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
    mut lanes: Query<
        (
            Entity,
            &Hovering,
            &Selected,
            &Lane,
            &LanePieces,
            &mut LaneVisualCue,
            &mut Transform,
        ),
        (
            Without<VertexVisualCue>,
            Or<(
                Changed<Hovering>,
                Changed<Selected>,
                Changed<Lane>,
            )>,
        ),
    >,
    mut vertices: Query<(&mut Hovering, &mut Selected), With<VertexVisualCue>>,
    mut materials: Query<&mut Handle<StandardMaterial>>,
    mut visibility: Query<&mut Visibility>,
    cursor: Query<Entity, With<Cursor>>,
    site_assets: Res<SiteAssets>,
    vm: Res<VerticesManagers>,
    level: Res<Option<SiteMapCurrentLevel>>,
) {
    let level = match level.as_ref() {
        Some(level) => level,
        None => {
            return;
        }
    };
    for (l, hovering, selected, lane, pieces, mut cue, mut tf) in &mut lanes {
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

                if let Some([(mut hover_v0, mut selected_v0), (mut hover_v1, mut selected_v1)]) =
                    vertices.get_many_mut([v0, v1]).ok()
                {
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

                if hovering.is_hovering {
                    set_visibility(cursor.single(), &mut visibility, false);
                }

                let (m, h) = if hovering.cue() && selected.cue() {
                    (&site_assets.hover_select_material, HOVERED_LANE_HEIGHT)
                } else if hovering.cue() {
                    (&site_assets.hover_material, HOVERED_LANE_HEIGHT)
                } else if selected.cue() {
                    (&site_assets.select_material, SELECTED_LANE_HEIGHT)
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

#[derive(Component)]
pub struct DefaultVisualCue;

pub fn update_floor_and_wall_visual_cues(
    floors: Query<&Hovering, With<FloorVisualCue>>,
    walls: Query<&Hovering, With<WallVisualCue>>,
    everything_else: Query<&Hovering, With<DefaultVisualCue>>,
    cursor: Query<Entity, With<Cursor>>,
    mut visibility: Query<&mut Visibility>,
) {
    for hovering in floors
        .iter()
        .chain(walls.iter())
        .chain(everything_else.iter())
    {
        if hovering.cue() {
            if let Some(mut v) = visibility.get_mut(cursor.single()).ok() {
                v.is_visible = true;
            }
        }
    }
}
