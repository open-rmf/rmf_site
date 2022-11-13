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
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use rmf_site_format::{DoorMarker, DoorType, Edge, Category, DEFAULT_LEVEL_HEIGHT};

pub const DOOR_CUE_HEIGHT: f32 = 0.004;
pub const DOOR_STOP_LINE_THICKNESS: f32 = 0.01;
pub const DOOR_STOP_LINE_LENGTH: f32 = 3.0 * DEFAULT_DOOR_THICKNESS;
pub const DOOR_SWEEP_THICKNESS: f32 = 0.05;

#[derive(Debug, Clone, Copy, Component)]
pub struct DoorSegments {
    // TODO(MXG): When it's time to animate the doors we should replace this
    // with an enum for the different possible door types: Single/Double Swing/Sliding
    pub body: Entity,
    pub cue_inner: Entity,
    pub cue_outline: Entity,
}

fn make_door_visuals(
    entity: Entity,
    edge: &Edge<Entity>,
    anchors: &AnchorParams,
    kind: &DoorType,
) -> (Transform, Transform, Mesh, Mesh) {
    let p_start = anchors.point_in_parent_frame_of(edge.left(), Category::Door, entity).unwrap();
    let p_end = anchors.point_in_parent_frame_of(edge.right(), Category::Door, entity).unwrap();

    let dp = p_start - p_end;
    let length = dp.length();
    let yaw = (-dp.x).atan2(dp.y);
    let center = (p_start + p_end) / 2.0;

    let (inner, outline) = make_door_cues(length, kind);
    (
        Transform {
            translation: Vec3::new(center.x, center.y, 0.),
            rotation: Quat::from_rotation_z(yaw),
            ..default()
        },
        Transform {
            translation: Vec3::new(0., 0., DEFAULT_LEVEL_HEIGHT / 2.0),
            scale: Vec3::new(DEFAULT_DOOR_THICKNESS, length, DEFAULT_LEVEL_HEIGHT),
            ..default()
        },
        inner,
        outline,
    )
}

fn door_slide_stop_line(y: f32) -> MeshBuffer {
    let x_span = DOOR_STOP_LINE_LENGTH;
    line_stroke_mesh(
        Vec3::new(-x_span, y, DOOR_CUE_HEIGHT),
        Vec3::new(x_span, y, DOOR_CUE_HEIGHT),
        DOOR_STOP_LINE_THICKNESS,
    )
}

fn door_slide_arrow(start: f32, stop: f32, sign: f32) -> MeshBuffer {
    let x_max = DOOR_STOP_LINE_LENGTH;
    let tip = DEFAULT_DOOR_THICKNESS;
    let handle_thickness = DEFAULT_DOOR_THICKNESS / 3.0;
    flat_arrow_mesh_between(
        Vec3::new(sign * (x_max - 2.0 / 3.0 * tip), start, DOOR_CUE_HEIGHT),
        Vec3::new(sign * (x_max - 2.0 / 3.0 * tip), stop, DOOR_CUE_HEIGHT),
        handle_thickness,
        tip,
        tip,
    )
}

fn door_slide_arrows(start: f32, stop: f32) -> MeshBuffer {
    door_slide_arrow(start, stop, -1.0).merge_with(door_slide_arrow(start, stop, 1.0))
}

fn door_swing_arc(door_width: f32, door_count: u32, pivot_on: Side, swing: Swing) -> MeshBuffer {
    let pivot = pivot_on.sign() * door_width / 2.0;
    let pivot = Vec3::new(0.0, pivot, DOOR_CUE_HEIGHT);
    let door_width = door_width / door_count as f32;
    let (initial_angle, sweep) = swing.swing_on_pivot(pivot_on);
    flat_arc(
        pivot,
        door_width,
        DOOR_SWEEP_THICKNESS,
        initial_angle,
        sweep,
        0.5,
    )
    .merge_with(line_stroke_away_from(
        pivot + pivot_on.sign() * DOOR_STOP_LINE_THICKNESS / 2.0 * Vec3::Y,
        initial_angle,
        door_width,
        DOOR_STOP_LINE_THICKNESS,
    ))
    .merge_with(line_stroke_away_from(
        pivot + pivot_on.sign() * DOOR_STOP_LINE_THICKNESS / 2.0 * Vec3::Y,
        initial_angle + sweep,
        door_width,
        DOOR_STOP_LINE_THICKNESS,
    ))
}

fn make_door_cues(door_width: f32, kind: &DoorType) -> (Mesh, Mesh) {
    match kind {
        DoorType::SingleSliding(door) => {
            let start =
                door.towards.opposite().sign() * (door_width - DOOR_STOP_LINE_THICKNESS) / 2.0;
            let stop = door.towards.sign() * (door_width - DOOR_STOP_LINE_THICKNESS) / 2.0;
            door_slide_stop_line(-door_width / 2.0)
                .merge_with(door_slide_stop_line(door_width / 2.0))
                .merge_with(door_slide_arrows(start, stop))
                .into_mesh_and_outline()
        }
        DoorType::DoubleSliding(door) => {
            let left = (door_width - DOOR_STOP_LINE_THICKNESS) / 2.0;
            let mid = door.compute_offset(door_width);
            let right = -(door_width - DOOR_STOP_LINE_THICKNESS) / 2.0;
            let tweak = DOOR_STOP_LINE_THICKNESS / 2.0;

            door_slide_stop_line(left)
                .merge_with(door_slide_stop_line(mid))
                .merge_with(door_slide_stop_line(right))
                .merge_with(door_slide_arrows(mid + tweak, left - tweak))
                .merge_with(door_slide_arrows(mid - tweak, right + tweak))
                .into_mesh_and_outline()
        }
        DoorType::SingleSwing(door) => {
            door_swing_arc(door_width, 1, door.pivot_on, door.swing).into_mesh_and_outline()
        }
        DoorType::DoubleSwing(door) => door_swing_arc(door_width, 2, Side::Left, door.swing)
            .merge_with(door_swing_arc(door_width, 2, Side::Right, door.swing))
            .into_mesh_and_outline(),
        _ => {
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
            mesh.set_indices(Some(Indices::U32(vec![])));
            (mesh.clone(), mesh)
        }
    }
}

pub fn add_door_visuals(
    mut commands: Commands,
    new_doors: Query<(Entity, &Edge<Entity>, &DoorType, Option<&Visibility>), (
        Or<(Added<DoorType>, Added<Edge<Entity>>)>,
        Without<DoorSegments>,
    )>,
    anchors: AnchorParams,
    mut dependents: Query<&mut AnchorDependents>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, edge, kind, visibility) in &new_doors {
        let (pose_tf, shape_tf, cue_inner_mesh, cue_outline_mesh) =
            make_door_visuals(e, edge, &anchors, kind);

        let mut commands = commands.entity(e);
        let (body, cue_inner, cue_outline) = commands.add_children(|parent| {
            let body = parent
                .spawn_bundle(PbrBundle {
                    mesh: assets.box_mesh.clone(),
                    material: assets.door_body_material.clone(),
                    transform: shape_tf,
                    ..default()
                })
                .insert(Selectable::new(e))
                .id();

            let cue_inner = parent
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(cue_inner_mesh),
                    material: assets.translucent_white.clone(),
                    ..default()
                })
                .id();

            let cue_outline = parent
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(cue_outline_mesh),
                    material: assets.translucent_black.clone(),
                    ..default()
                })
                .id();

            (body, cue_inner, cue_outline)
        });

        // Level doors for lifts may have already been given a Visibility
        // component upon creation, in which case we should respect whatever
        // value was set for it.
        let is_visible = if let Some(v) = visibility {
            v.is_visible
        } else {
            true
        };

        commands
            .insert_bundle(SpatialBundle {
                transform: pose_tf,
                visibility: Visibility{ is_visible },
                ..default()
            })
            .insert(DoorSegments {
                body,
                cue_inner,
                cue_outline,
            })
            .insert(Category::Door)
            .insert(EdgeLabels::LeftRight);

        for anchor in edge.array() {
            if let Ok(mut dep) = dependents.get_mut(anchor) {
                dep.dependents.insert(e);
            }
        }
    }
}

fn update_door_visuals(
    entity: Entity,
    edge: &Edge<Entity>,
    kind: &DoorType,
    segments: &DoorSegments,
    anchors: &AnchorParams,
    transforms: &mut Query<&mut Transform>,
    mesh_handles: &mut Query<&mut Handle<Mesh>>,
    mesh_assets: &mut ResMut<Assets<Mesh>>,
) {
    let (pose_tf, shape_tf, cue_inner_mesh, cue_outline_mesh) =
        make_door_visuals(entity, edge, anchors, kind);
    let mut door_transform = transforms.get_mut(entity).unwrap();
    *door_transform = pose_tf;
    let mut shape_transform = transforms.get_mut(segments.body).unwrap();
    *shape_transform = shape_tf;
    let mut cue_inner = mesh_handles.get_mut(segments.cue_inner).unwrap();
    *cue_inner = mesh_assets.add(cue_inner_mesh);
    let mut cue_outline = mesh_handles.get_mut(segments.cue_outline).unwrap();
    *cue_outline = mesh_assets.add(cue_outline_mesh);
}

pub fn update_changed_door(
    doors: Query<
        (Entity, &Edge<Entity>, &DoorType, &DoorSegments),
        Or<(Changed<Edge<Entity>>, Changed<DoorType>)>,
    >,
    anchors: AnchorParams,
    mut transforms: Query<&mut Transform>,
    mut mesh_handles: Query<&mut Handle<Mesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
) {
    for (entity, edge, kind, segments) in &doors {
        update_door_visuals(
            entity,
            edge,
            kind,
            segments,
            &anchors,
            &mut transforms,
            &mut mesh_handles,
            &mut mesh_assets,
        );
    }
}

pub fn update_door_for_changed_anchor(
    doors: Query<(Entity, &Edge<Entity>, &DoorType, &DoorSegments)>,
    anchors: AnchorParams,
    changed_anchors: Query<&AnchorDependents, (With<Anchor>, Changed<GlobalTransform>)>,
    mut transforms: Query<&mut Transform>,
    mut mesh_handles: Query<&mut Handle<Mesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((entity, edge, kind, segments)) = doors.get(*dependent).ok() {
                update_door_visuals(
                    entity,
                    edge,
                    kind,
                    segments,
                    &anchors,
                    &mut transforms,
                    &mut mesh_handles,
                    &mut mesh_assets,
                );
            }
        }
    }
}
