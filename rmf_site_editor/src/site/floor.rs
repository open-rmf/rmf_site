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
    interaction::Selectable,
    shapes::*,
    site::*,
    RecencyRanking,
};
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
                error!("Failed to find anchor {anchor:?} used by a path");
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
    let mut reference_positions = Vec::new();
    for anchor in &anchor_path.0 {
        let p = match anchors.point_in_parent_frame_of(*anchor, Category::Floor, entity) {
            Ok(a) => a,
            Err(_) => {
                error!("Failed to find anchor {anchor:?} used by a path");
                valid = false;
                continue;
            }
        };

        reference_positions.push(p.to_array());
        if first {
            first = false;
            builder.begin(point(p.x, p.y));
        } else {
            builder.line_to(point(p.x, p.y));
        }
    }
    let outline_buffer = make_closed_path_outline(reference_positions);

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
                error!("Failed to render floor: {err}");
                return make_fallback_floor_mesh_near_path(entity, anchor_path, anchors);
            }
            _ => {}
        }
    }

    let positions: Vec<[f32; 3]> = buffers.vertices.iter().map(|v| [v.x, v.y, 0.]).collect();
    let normals: Vec<[f32; 3]> = buffers.vertices.iter().map(|_| [0., 0., 1.]).collect();
    let uv: Vec<[f32; 2]> = buffers.vertices.iter().map(|v| [v.x, v.y]).collect();
    for i in 0..buffers.indices.len() / 3 {
        let i1 = 3 * i + 1;
        let i2 = 3 * i + 2;
        buffers.indices.swap(i1, i2);
    }
    let indices = buffers.indices.drain(..).map(|v| v as u32).collect();

    MeshBuffer::new(positions, normals, indices)
        .with_uv(uv)
        .merge_with(outline_buffer)
        .into()
}

fn floor_height(rank: Option<&RecencyRank<FloorMarker>>) -> f32 {
    rank.map(|r| r.proportion() * (LANE_LAYER_START - FLOOR_LAYER_START) + FLOOR_LAYER_START)
        .unwrap_or(FLOOR_LAYER_START)
}

#[inline]
fn floor_material(
    specific: Option<&LayerVisibility>,
    general: Option<(&GlobalFloorVisibility, &RecencyRanking<DrawingMarker>)>,
) -> StandardMaterial {
    let alpha = specific.copied()
        .unwrap_or_else(
            || general.map(|(v, r)| {
                if r.is_empty() {
                    &v.without_drawings
                } else {
                    &v.general
                }
            }).copied()
            .unwrap_or(LayerVisibility::Opaque)
        ).alpha();

    Color::rgba(0.3, 0.3, 0.3, alpha).into()
}

pub fn add_floor_visuals(
    mut commands: Commands,
    floors: Query<
        (
            Entity,
            &Path<Entity>,
            Option<&RecencyRank<FloorMarker>>,
            Option<&LayerVisibility>,
            Option<&Parent>,
        ),
        Added<FloorMarker>,
    >,
    anchors: AnchorParams,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    default_floor_vis: Query<(&GlobalFloorVisibility, &RecencyRanking<DrawingMarker>)>,
) {
    for (e, new_floor, rank, vis, parent) in &floors {
        let mesh = make_floor_mesh(e, new_floor, &anchors);
        let mut cmd = commands.entity(e);
        let height = floor_height(rank);
        let default = parent.map(|p| default_floor_vis.get(p.get()).ok()).flatten();
        let material = materials.add(floor_material(vis, default));

        let mesh_entity_id = cmd
            .insert(SpatialBundle {
                transform: Transform::from_xyz(0.0, 0.0, height),
                ..Default::default()
            })
            .add_children(|p| {
                p.spawn(PbrBundle {
                    mesh: meshes.add(mesh),
                    // TODO(MXG): load the user-specified texture when one is given
                    material,
                    ..Default::default()
                })
                .insert(Selectable::new(e))
                .id()
            });

        cmd.insert(FloorSegments {
            mesh: mesh_entity_id,
        })
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
    changed_rank: Query<(Entity, &RecencyRank<FloorMarker>), Changed<RecencyRank<FloorMarker>>>,
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

#[inline]
fn iter_update_floor_visibility<'a>(
    iter: impl Iterator<Item = (Option<&'a LayerVisibility>, Option<&'a Parent>, &'a FloorSegments)>,
    material_handles: &Query<&Handle<StandardMaterial>>,
    material_assets: &mut ResMut<Assets<StandardMaterial>>,
    default_floor_vis: &Query<(&GlobalFloorVisibility, &RecencyRanking<DrawingMarker>)>,
) {
    for (vis, parent, segments) in iter {
        if let Ok(handle) = material_handles.get(segments.mesh) {
            if let Some(mat) = material_assets.get_mut(handle) {
                let default = parent.map(|p| default_floor_vis.get(p.get()).ok()).flatten();
                *mat = floor_material(vis, default);
            }
        }
    }
}

// TODO(luca) RemovedComponents is brittle, maybe wrap component in an option?
pub fn update_floor_visibility(
    changed_floors: Query<Entity, Or<(Changed<LayerVisibility>, Changed<Parent>)>>,
    removed_vis: RemovedComponents<LayerVisibility>,
    all_floors: Query<(Option<&LayerVisibility>, Option<&Parent>, &FloorSegments)>,
    material_handles: Query<&Handle<StandardMaterial>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    default_floor_vis: Query<(&GlobalFloorVisibility, &RecencyRanking<DrawingMarker>)>,
    changed_default_floor_vis: Query<
        &Children,
        Or<(Changed<GlobalFloorVisibility>, Changed<RecencyRanking<DrawingMarker>>)>,
    >,
) {
    iter_update_floor_visibility(
        changed_floors.iter().filter_map(|e| all_floors.get(e).ok()),
        &material_handles,
        &mut material_assets,
        &default_floor_vis,
    );

    iter_update_floor_visibility(
        removed_vis.iter().filter_map(|e| all_floors.get(e).ok()),
        &material_handles,
        &mut material_assets,
        &default_floor_vis,
    );

    for children in &changed_default_floor_vis {
        iter_update_floor_visibility(
            children.iter().filter_map(|e| all_floors.get(*e).ok()),
            &material_handles,
            &mut material_assets,
            &default_floor_vis,
        );
    }
}
