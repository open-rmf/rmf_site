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

use crate::{interaction::Selectable, shapes::*, site::*, RecencyRanking};
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
use rmf_site_format::{FloorMarker, Path, Texture};

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

fn make_floor_mesh(
    entity: Entity,
    anchor_path: &Path<Entity>,
    texture: &Texture,
    anchors: &AnchorParams,
) -> Mesh {
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

    let texture_width = texture.width.unwrap_or(1.0);
    let texture_height = texture.height.unwrap_or(1.0);
    let positions: Vec<[f32; 3]> = buffers.vertices.iter().map(|v| [v.x, v.y, 0.]).collect();
    let normals: Vec<[f32; 3]> = buffers.vertices.iter().map(|_| [0., 0., 1.]).collect();
    let uv: Vec<[f32; 2]> = buffers
        .vertices
        .iter()
        .map(|v| [v.x / texture_width, v.y / texture_height])
        .collect();
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
fn floor_transparency(
    specific: Option<&LayerVisibility>,
    general: Option<(&GlobalFloorVisibility, &RecencyRanking<DrawingMarker>)>,
) -> (Color, AlphaMode) {
    let alpha = specific
        .copied()
        .unwrap_or_else(|| {
            general
                .map(|(v, r)| {
                    if r.is_empty() {
                        &v.without_drawings
                    } else {
                        &v.general
                    }
                })
                .copied()
                .unwrap_or(LayerVisibility::Opaque)
        })
        .alpha();

    let alpha_mode = if alpha < 1.0 {
        AlphaMode::Blend
    } else {
        AlphaMode::Opaque
    };
    (*Color::default().set_a(alpha), alpha_mode)
}

pub fn add_floor_visuals(
    mut commands: Commands,
    floors: Query<
        (
            Entity,
            &Path<Entity>,
            &Affiliation<Entity>,
            Option<&RecencyRank<FloorMarker>>,
            Option<&LayerVisibility>,
            Option<&Parent>,
        ),
        Added<FloorMarker>,
    >,
    anchors: AnchorParams,
    textures: Query<(Option<&Handle<Image>>, &Texture)>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    default_floor_vis: Query<(&GlobalFloorVisibility, &RecencyRanking<DrawingMarker>)>,
) {
    for (e, new_floor, texture_source, rank, vis, parent) in &floors {
        let (base_color_texture, texture) = from_texture_source(texture_source, &textures);

        let mesh = make_floor_mesh(e, new_floor, &texture, &anchors);
        let height = floor_height(rank);
        let default_vis = parent
            .map(|p| default_floor_vis.get(p.get()).ok())
            .flatten();
        let (base_color, alpha_mode) = floor_transparency(vis, default_vis);
        let material = materials.add(StandardMaterial {
            base_color_texture,
            base_color,
            alpha_mode,
            perceptual_roughness: 0.089,
            ..default()
        });

        let mesh_entity_id = commands
            .spawn(PbrBundle {
                mesh: meshes.add(mesh),
                material,
                ..default()
            })
            .insert(Selectable::new(e))
            .id();

        commands
            .entity(e)
            .insert(SpatialBundle {
                transform: Transform::from_xyz(0.0, 0.0, height),
                ..default()
            })
            .insert(FloorSegments {
                mesh: mesh_entity_id,
            })
            .insert(Category::Floor)
            .insert(PathBehavior::for_floor())
            .add_child(mesh_entity_id);

        for anchor in &new_floor.0 {
            let mut deps = dependents.get_mut(*anchor).unwrap();
            deps.insert(e);
        }
    }
}

pub fn update_changed_floor_ranks(
    changed_rank: Query<(Entity, &RecencyRank<FloorMarker>), Changed<RecencyRank<FloorMarker>>>,
    mut transforms: Query<&mut Transform>,
) {
    for (e, rank) in &changed_rank {
        if let Ok(mut tf) = transforms.get_mut(e) {
            tf.translation.z = floor_height(Some(rank));
        }
    }
}

pub fn update_floors_for_moved_anchors(
    floors: Query<(Entity, &FloorSegments, &Path<Entity>, &Affiliation<Entity>), With<FloorMarker>>,
    anchors: AnchorParams,
    textures: Query<(Option<&Handle<Image>>, &Texture)>,
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
            if let Some((e, segments, path, texture_source)) = floors.get(*dependent).ok() {
                let (_, texture) = from_texture_source(texture_source, &textures);
                if let Ok(mut mesh) = mesh_handles.get_mut(segments.mesh) {
                    *mesh = mesh_assets.add(make_floor_mesh(e, path, &texture, &anchors));
                }
            }
        }
    }
}

pub fn update_floors(
    floors: Query<(&FloorSegments, &Path<Entity>, &Affiliation<Entity>), With<FloorMarker>>,
    changed_floors: Query<
        Entity,
        (
            With<FloorMarker>,
            Or<(Changed<Affiliation<Entity>>, Changed<Path<Entity>>)>,
        ),
    >,
    changed_texture_sources: Query<
        &Members,
        (With<Group>, Or<(Changed<Handle<Image>>, Changed<Texture>)>),
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut mesh_handles: Query<&mut Handle<Mesh>>,
    material_handles: Query<&Handle<StandardMaterial>>,
    anchors: AnchorParams,
    textures: Query<(Option<&Handle<Image>>, &Texture)>,
) {
    for e in changed_floors.iter().chain(
        changed_texture_sources
            .iter()
            .flat_map(|members| members.iter().cloned()),
    ) {
        let Ok((segment, path, texture_source)) = floors.get(e) else {
            continue;
        };
        let (base_color_texture, texture) = from_texture_source(texture_source, &textures);
        if let Ok(mut mesh) = mesh_handles.get_mut(segment.mesh) {
            if let Ok(material) = material_handles.get(segment.mesh) {
                *mesh = meshes.add(make_floor_mesh(e, path, &texture, &anchors));
                if let Some(mut material) = materials.get_mut(material) {
                    material.base_color_texture = base_color_texture;
                }
            }
        }
    }
}

#[inline]
fn iter_update_floor_visibility<'a>(
    iter: impl Iterator<
        Item = (
            Option<&'a LayerVisibility>,
            Option<&'a Parent>,
            &'a FloorSegments,
        ),
    >,
    material_handles: &Query<&Handle<StandardMaterial>>,
    material_assets: &mut ResMut<Assets<StandardMaterial>>,
    default_floor_vis: &Query<(&GlobalFloorVisibility, &RecencyRanking<DrawingMarker>)>,
) {
    for (vis, parent, segments) in iter {
        if let Ok(handle) = material_handles.get(segments.mesh) {
            if let Some(mat) = material_assets.get_mut(handle) {
                let default_vis = parent
                    .map(|p| default_floor_vis.get(p.get()).ok())
                    .flatten();
                let (base_color, alpha_mode) = floor_transparency(vis, default_vis);
                mat.base_color = base_color;
                mat.alpha_mode = alpha_mode;
            }
        }
    }
}

// TODO(luca) RemovedComponents is brittle, maybe wrap component in an option?
pub fn update_floor_visibility(
    changed_floors: Query<Entity, Or<(Changed<LayerVisibility>, Changed<Parent>)>>,
    mut removed_vis: RemovedComponents<LayerVisibility>,
    all_floors: Query<(Option<&LayerVisibility>, Option<&Parent>, &FloorSegments)>,
    material_handles: Query<&Handle<StandardMaterial>>,
    mut material_assets: ResMut<Assets<StandardMaterial>>,
    default_floor_vis: Query<(&GlobalFloorVisibility, &RecencyRanking<DrawingMarker>)>,
    changed_default_floor_vis: Query<
        &Children,
        Or<(
            Changed<GlobalFloorVisibility>,
            Changed<RecencyRanking<DrawingMarker>>,
        )>,
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
