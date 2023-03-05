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

use crate::{interaction::Selectable, shapes::*, site::*};
use bevy::{
    math::Affine3A,
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use lyon::{
    math::point,
    path::Path as LyonPath,
    tessellation::{geometry_builder::simple_builder, *},
};
use rmf_site_format::{FloorMarker, Path};

const DEFAULT_FLOOR_SEMI_TRANSPARENCY: f32 = 0.2;

// TODO(MXG): Should we make this more general? Should we be able to apply this
// component to Drawings as well?
#[derive(Debug, Clone, Copy, Resource, Component)]
pub enum FloorVisibility {
    /// The floors are fully opaque. This is the default when no drawing is
    /// present.
    Opaque,
    /// Make the floors semi-transparent. This is useful for allowing drawings
    /// to be visible undearneath them. When a drawing is added to the scene,
    /// the floors will automatically change to Alpha(0.1).
    Alpha(f32),
    /// The floors are fully hidden.
    Hidden,
}

// TODO(MXG): Should this trait be more general?
pub trait Cycle {
    type Value;
    fn next(&self) -> Self::Value;
    fn label(&self) -> &'static str;
}

impl FloorVisibility {
    pub fn new_semi_transparent() -> Self {
        FloorVisibility::Alpha(DEFAULT_FLOOR_SEMI_TRANSPARENCY)
    }

    pub fn alpha(&self) -> f32 {
        match self {
            FloorVisibility::Opaque => 1.0,
            FloorVisibility::Alpha(a) => *a,
            FloorVisibility::Hidden => 0.0,
        }
    }
}

impl Cycle for FloorVisibility {
    type Value = Self;

    /// Cycle to the next visibility option
    fn next(&self) -> FloorVisibility {
        match self {
            FloorVisibility::Opaque => FloorVisibility::new_semi_transparent(),
            FloorVisibility::Alpha(_) => FloorVisibility::Hidden,
            FloorVisibility::Hidden => FloorVisibility::Opaque,
        }
    }

    fn label(&self) -> &'static str {
        match self {
            FloorVisibility::Opaque => "opaque",
            FloorVisibility::Alpha(_) => "semi-transparent",
            FloorVisibility::Hidden => "hidden",
        }
    }
}

impl Cycle for Option<FloorVisibility> {
    type Value = Self;
    fn next(&self) -> Self {
        match self {
            Some(v) => {
                match v {
                    FloorVisibility::Hidden => None,
                    _ => Some(v.next()),
                }
            }
            None => Some(FloorVisibility::Opaque),
        }
    }

    fn label(&self) -> &'static str {
        match self {
            Some(v) => v.label(),
            None => "global default",
        }
    }
}

impl Default for FloorVisibility {
    fn default() -> Self {
        FloorVisibility::Opaque
    }
}

pub const FALLBACK_FLOOR_SIZE: f32 = 0.1;
pub const FLOOR_LAYER_START: f32 = DRAWING_LAYER_START + 0.001;

#[derive(Debug, Clone, Copy, Component)]
pub struct FloorSegments {
    mesh: Entity,
}

fn make_fallback_floor_mesh(p: Vec3) -> Mesh {
    make_flat_square_mesh(1.0)
        .transform_by(Affine3A::from_scale_rotation_translation(
            Vec3::splat(0.5),
            Quat::from_rotation_z(0.0),
            p,
        ))
        .into()
}

fn make_fallback_floor_mesh_at_avg(positions: Vec<Vec3>) -> Mesh {
    let p = positions.iter().fold(Vec3::ZERO, |sum, x| sum + *x) / positions.len() as f32;
    return make_fallback_floor_mesh(p);
}

fn make_fallback_floor_mesh_near_path(
    entity: Entity,
    path: &Path<Entity>,
    anchors: &AnchorParams,
) -> Mesh {
    let mut positions: Vec<Vec3> = Vec::new();
    for anchor in path.iter() {
        if let Ok(p) = anchors.point_in_parent_frame_of(*anchor, Category::Floor, entity) {
            positions.push(p);
        }
    }
    return make_fallback_floor_mesh_at_avg(positions);
}

fn make_floor_mesh(entity: Entity, anchor_path: &Path<Entity>, anchors: &AnchorParams) -> Mesh {
    if anchor_path.len() == 0 {
        return Mesh::new(PrimitiveTopology::TriangleList);
    } else if anchor_path.len() == 1 {
        let p = anchors
            .point_in_parent_frame_of(anchor_path[0], Category::Floor, entity)
            .unwrap_or(Vec3::ZERO);
        return make_fallback_floor_mesh(p);
    } else if anchor_path.len() == 2 {
        let mut positions: Vec<Vec3> = Vec::new();
        let mut valid = true;
        for anchor in anchor_path.iter() {
            if let Ok(p) = anchors.point_in_parent_frame_of(*anchor, Category::Floor, entity) {
                positions.push(p);
            } else {
                println!("DEV ERROR: Failed to find anchor {anchor:?} used by a path");
                valid = false;
            }
        }
        if !valid {
            return make_fallback_floor_mesh_at_avg(positions);
        }

        let tf = line_stroke_transform(&positions[0], &positions[1], FALLBACK_FLOOR_SIZE);
        return make_flat_square_mesh(1.0)
            .transform_by(tf.compute_affine())
            .into();
    }

    let mut builder = LyonPath::builder();
    let mut first = true;
    let mut valid = true;
    for anchor in &anchor_path.0 {
        let p = match anchors.point_in_parent_frame_of(*anchor, Category::Floor, entity) {
            Ok(a) => a,
            Err(_) => {
                println!("DEV ERROR: Failed to find anchor {anchor:?} used by a path");
                valid = false;
                continue;
            }
        };

        if first {
            first = false;
            builder.begin(point(p.x, p.y));
        } else {
            builder.line_to(point(p.x, p.y));
        }
    }

    if !valid {
        return make_fallback_floor_mesh_near_path(entity, anchor_path, anchors);
    }

    builder.close();
    let path = builder.build();

    let mut buffers = VertexBuffers::new();
    {
        let mut vertex_builder = simple_builder(&mut buffers);
        let mut tessellator = FillTessellator::new();
        let result = tessellator.tessellate_path(
            path.as_slice(),
            &FillOptions::default(),
            &mut vertex_builder,
        );

        match result {
            Err(err) => {
                println!("Failed to render floor: {err}");
                return make_fallback_floor_mesh_near_path(entity, anchor_path, anchors);
            }
            _ => {}
        }
    }

    let positions: Vec<[f32; 3]> = buffers.vertices.iter().map(|v| [v.x, v.y, 0.]).collect();

    let normals: Vec<[f32; 3]> = buffers.vertices.iter().map(|_| [0., 0., 1.]).collect();

    let uv: Vec<[f32; 2]> = buffers.vertices.iter().map(|v| [v.x, v.y]).collect();

    let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
    for i in 0..buffers.indices.len() / 3 {
        let i1 = 3 * i + 1;
        let i2 = 3 * i + 2;
        buffers.indices.swap(i1, i2);
    }
    let indices = buffers.indices.drain(..).map(|v| v as u32).collect();
    mesh.set_indices(Some(Indices::U32(indices)));
    mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, positions);
    mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, normals);
    mesh.insert_attribute(Mesh::ATTRIBUTE_UV_0, uv);

    mesh
}

fn floor_height(rank: Option<&RecencyRank<FloorMarker>>) -> f32 {
    rank
    .map(|r| r.proportion() * (LANE_LAYER_START - FLOOR_LAYER_START) + FLOOR_LAYER_START)
    .unwrap_or(FLOOR_LAYER_START)
}

fn floor_material(
    specific: Option<&FloorVisibility>,
    general: &FloorVisibility,
) -> StandardMaterial {
    let alpha = specific.map(|s| s.alpha()).unwrap_or(general.alpha());
    Color::rgba(0.3, 0.3, 0.3, alpha).into()
}

pub fn add_floor_visuals(
    mut commands: Commands,
    floors: Query<(Entity, &Path<Entity>, Option<&RecencyRank<FloorMarker>>, Option<&FloorVisibility>), Added<FloorMarker>>,
    anchors: AnchorParams,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    default_floor_visibility: Res<FloorVisibility>,
) {
    for (e, new_floor, rank, vis) in &floors {
        let mesh = make_floor_mesh(e, new_floor, &anchors);
        let mut cmd = commands.entity(e);
        let height = floor_height(rank);
        let material = materials.add(
            floor_material(vis, default_floor_visibility.as_ref())
        );

        let mesh_entity_id = cmd
            .insert(SpatialBundle {
                transform: Transform::from_xyz(0.0, 0.0, height),
                ..default()
            })
            .add_children(|p| {
                p
                .spawn(PbrBundle {
                    mesh: meshes.add(mesh),
                    // TODO(MXG): load the user-specified texture when one is given
                    material,
                    ..default()
                })
                .insert(Selectable::new(e))
                .id()
            });

        cmd
            .insert(FloorSegments { mesh: mesh_entity_id })
            .insert(Category::Floor)
            .insert(PathBehavior::for_floor());

        for anchor in &new_floor.0 {
            let mut deps = dependents.get_mut(*anchor).unwrap();
            deps.insert(e);
        }
    }
}

pub fn update_changed_floor(
    changed_path: Query<
        (Entity, &FloorSegments, &Path<Entity>),
        (Changed<Path<Entity>>, With<FloorMarker>),
    >,
    changed_rank: Query<
        (Entity, &RecencyRank<FloorMarker>),
        Changed<RecencyRank<FloorMarker>>,
    >,
    anchors: AnchorParams,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut transforms: Query<&mut Transform>,
    mut mesh_handles: Query<&mut Handle<Mesh>>,
) {
    for (e, segments, path) in &changed_path {
        if let Ok(mut mesh) = mesh_handles.get_mut(segments.mesh) {
            *mesh = mesh_assets.add(make_floor_mesh(e, path, &anchors));
        }
        // TODO(MXG): Update texture once we support textures
    }

    for (e, rank) in &changed_rank {
        if let Ok(mut tf) = transforms.get_mut(e) {
            tf.translation.z = floor_height(Some(rank));
        }
    }
}

pub fn update_floor_for_moved_anchors(
    floors: Query<(Entity, &FloorSegments, &Path<Entity>), With<FloorMarker>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut mesh_handles: Query<&mut Handle<Mesh>>,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Some((e, segments, path)) = floors.get(*dependent).ok() {
                if let Ok(mut mesh) = mesh_handles.get_mut(segments.mesh) {
                    *mesh = mesh_assets.add(make_floor_mesh(e, path, &anchors));
                }
            }
        }
    }
}

fn iter_update_floor_visibility<'a>(
    iter: impl Iterator<Item=(Option<&'a FloorVisibility>, &'a FloorSegments)>,
    material_handles: &Query<&Handle<StandardMaterial>>,
    material_assets: &mut ResMut<Assets<StandardMaterial>>,
    default_floor_vis: &FloorVisibility,
) {
    for (vis, segments) in iter {
        if let Ok(handle) = material_handles.get(segments.mesh) {
            if let Some(mat) = material_assets.get_mut(handle) {
                *mat = floor_material(vis, &default_floor_vis);
            }
        }
    }
}

pub fn update_floor_visibility(
    changed_floors: Query<(Option<&FloorVisibility>, &FloorSegments), Changed<FloorVisibility>>,
    removed_vis: RemovedComponents<FloorVisibility>,
    all_floors: Query<(Option<&FloorVisibility>, &FloorSegments)>,
    material_handles: Query<&Handle<StandardMaterial>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    default_floor_vis: Res<FloorVisibility>,
) {
    if default_floor_vis.is_changed() {
        iter_update_floor_visibility(
            all_floors.iter(),
            &material_handles,
            &mut material_assets,
            &default_floor_vis,
        );
    } else {
        iter_update_floor_visibility(
            changed_floors.iter(),
            &material_handles,
            &mut material_assets,
            &default_floor_vis,
        );

        iter_update_floor_visibility(
            removed_vis.iter().filter_map(|e|
                all_floors.get(e).ok()
            ),
            &material_handles,
            &mut material_assets,
            &default_floor_vis,
        );
    };
}
