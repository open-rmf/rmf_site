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
use bevy::{math::Affine3A, prelude::*, render::mesh::PrimitiveTopology};
use geo::{
    geometry::{LineString, MultiPolygon, Polygon},
    BooleanOps, CoordsIter, TriangulateSpade,
};
use rmf_site_format::{FloorMarker, Path, Texture};

pub const FALLBACK_FLOOR_SIZE: f32 = 0.1;
pub const FLOOR_LAYER_START: f32 = DRAWING_LAYER_START + 0.001;

#[derive(Debug, Clone, Copy, Component)]
pub struct FloorSegments {
    pub mesh: Entity,
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
    lifts: &Query<(&Transform, &LiftCabin<Entity>)>,
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

    let mut reference_positions = Vec::new();
    let mut valid = true;
    for anchor in &anchor_path.0 {
        match anchors.point_in_parent_frame_of(*anchor, Category::Floor, entity) {
            Ok(p) => reference_positions.push(p.to_array()),
            Err(_) => {
                error!("Failed to find anchor {anchor:?} used by a path");
                valid = false;
                continue;
            }
        };
    }
    let mut polygon = MultiPolygon::from(Polygon::new(
        LineString::from(
            reference_positions
                .iter()
                .map(|p| [p[0], p[1]])
                .collect::<Vec<_>>(),
        ),
        vec![],
    ));
    let outline_buffer = make_closed_path_outline(reference_positions);

    if !valid {
        return make_fallback_floor_mesh_near_path(entity, anchor_path, anchors);
    }
    // Subtract all the lift cabin AABBs
    for (tf, cabin) in lifts.iter() {
        let to_subtract = match cabin {
            LiftCabin::Rect(params) => {
                let w = params.thickness();
                let gap_for_door = |d: &Option<LiftCabinDoorPlacement<Entity>>| -> f32 {
                    d.map(|d| d.custom_gap.unwrap_or(params.gap()))
                        .unwrap_or(DEFAULT_CABIN_GAP)
                        + w
                };
                let aabb = params.aabb();
                let tf_cabin = *tf * Transform::from_translation(aabb.center.into());
                let front_gap = gap_for_door(&params.front_door);
                let right_gap = gap_for_door(&params.right_door);
                let back_gap = gap_for_door(&params.back_door);
                let left_gap = gap_for_door(&params.left_door);
                let he = aabb.half_extents;
                let he0 = Vec3::new(he.x + front_gap, he.y + left_gap, 0.0);
                let he1 = Vec3::new(he.x + front_gap, -he.y - right_gap, 0.0);
                let he2 = Vec3::new(-he.x - back_gap, -he.y - right_gap, 0.0);
                let he3 = Vec3::new(-he.x - back_gap, he.y + left_gap, 0.0);
                let p0 = tf_cabin.transform_point(he0);
                let p1 = tf_cabin.transform_point(he1);
                let p2 = tf_cabin.transform_point(he2);
                let p3 = tf_cabin.transform_point(he3);
                Polygon::new(
                    LineString::from(vec![[p0.x, p0.y], [p1.x, p1.y], [p2.x, p2.y], [p3.x, p3.y]]),
                    vec![],
                )
            } // When new lift types are added, add their footprint calculation here.
        };
        polygon = polygon.difference(&to_subtract.into());
    }
    let mut positions: Vec<[f32; 3]> = Vec::new();
    for polygon in polygon.iter() {
        let Ok(triangles) = polygon.constrained_triangulation(Default::default()) else {
            warn!("Failed triangulating lift floor hole");
            continue;
        };
        positions.reserve(triangles.len() * 3);
        for triangle in triangles.iter() {
            positions.extend(triangle.coords_iter().map(|v| [v.x, v.y, 0.]));
        }
    }

    let texture_width = texture.width.unwrap_or(1.0);
    let texture_height = texture.height.unwrap_or(1.0);
    let indices = (0..positions.len() as u32).collect();
    let normals: Vec<[f32; 3]> = positions.iter().map(|_| [0., 0., 1.]).collect();
    let uv: Vec<[f32; 2]> = positions
        .iter()
        .map(|v| [v[0] / texture_width, v[1] / texture_height])
        .collect();

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
    lifts: Query<(&Transform, &LiftCabin<Entity>)>,
) {
    for (e, new_floor, texture_source, rank, vis, parent) in &floors {
        let (base_color_texture, texture) = from_texture_source(texture_source, &textures);

        let mesh = make_floor_mesh(e, new_floor, &texture, &anchors, &lifts);
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
            metallic: 0.01,
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
    lifts: Query<(&Transform, &LiftCabin<Entity>)>,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Some((e, segments, path, texture_source)) = floors.get(*dependent).ok() {
                let (_, texture) = from_texture_source(texture_source, &textures);
                if let Ok(mut mesh) = mesh_handles.get_mut(segments.mesh) {
                    *mesh = mesh_assets.add(make_floor_mesh(e, path, &texture, &anchors, &lifts));
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
    lifts: Query<(&Transform, &LiftCabin<Entity>)>,
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
                *mesh = meshes.add(make_floor_mesh(e, path, &texture, &anchors, &lifts));
                if let Some(material) = materials.get_mut(material) {
                    material.base_color_texture = base_color_texture;
                }
            }
        }
    }
}

pub fn update_floors_for_changed_lifts(
    lifts: Query<(&Transform, &LiftCabin<Entity>)>,
    changed_lifts: Query<
        (),
        Or<(
            Changed<LiftCabin<Entity>>,
            (With<LiftCabin<Entity>>, Changed<GlobalTransform>),
        )>,
    >,
    removed_lifts: RemovedComponents<LiftCabin<Entity>>,
    floors: Query<(Entity, &FloorSegments, &Path<Entity>, &Affiliation<Entity>), With<FloorMarker>>,
    anchors: AnchorParams,
    textures: Query<(Option<&Handle<Image>>, &Texture)>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    mut mesh_handles: Query<&mut Handle<Mesh>>,
) {
    if changed_lifts.is_empty() && removed_lifts.is_empty() {
        return;
    }
    for (e, segments, path, texture_source) in floors.iter() {
        let (_, texture) = from_texture_source(texture_source, &textures);
        if let Ok(mut mesh) = mesh_handles.get_mut(segments.mesh) {
            *mesh = mesh_assets.add(make_floor_mesh(e, path, &texture, &anchors, &lifts));
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
        removed_vis.read().filter_map(|e| all_floors.get(e).ok()),
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
