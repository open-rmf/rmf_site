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

use crate::{issue::*, site::*};
use bevy::{
    ecs::{hierarchy::ChildOf, relationship::AncestorIter},
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
};
use rmf_site_format::{Category, DoorType, Edge, DEFAULT_LEVEL_HEIGHT};
use rmf_site_mesh::{
    flat_arc, flat_arrow_mesh_between, line_stroke_away_from, line_stroke_mesh, MeshBuffer, Radians,
};
use rmf_site_picking::{Hovered, Selectable};
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

pub const DOOR_CUE_HEIGHT: f32 = 0.004;
pub const DOOR_STOP_LINE_THICKNESS: f32 = 0.01;
pub const DOOR_STOP_LINE_LENGTH: f32 = 3.0 * DEFAULT_DOOR_THICKNESS;
pub const DOOR_SWEEP_THICKNESS: f32 = 0.05;
pub const DOUBLE_DOOR_GAP: f32 = 0.05;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum DoorBodyType {
    SingleSwing { body: Entity },
    DoubleSwing { left: Entity, right: Entity },
    SingleSliding { body: Entity },
    DoubleSliding { left: Entity, right: Entity },
    Model { body: Entity },
}

impl DoorBodyType {
    pub fn from_door_type(door_type: &DoorType, entities: &Vec<Entity>) -> Self {
        match door_type {
            DoorType::SingleSwing(_) => DoorBodyType::SingleSwing { body: entities[0] },
            DoorType::DoubleSwing(_) => DoorBodyType::DoubleSwing {
                left: entities[0],
                right: entities[1],
            },
            DoorType::SingleSliding(_) => DoorBodyType::SingleSliding { body: entities[0] },
            DoorType::DoubleSliding(_) => DoorBodyType::DoubleSliding {
                left: entities[0],
                right: entities[1],
            },
            DoorType::Model(_) => DoorBodyType::Model { body: entities[0] },
        }
    }

    pub fn entities(&self) -> Vec<Entity> {
        match self {
            DoorBodyType::SingleSwing { body }
            | DoorBodyType::SingleSliding { body }
            | DoorBodyType::Model { body } => {
                vec![*body]
            }
            DoorBodyType::DoubleSwing { left, right }
            | DoorBodyType::DoubleSliding { left, right } => vec![*left, *right],
        }
    }

    pub fn links(&self) -> Vec<&str> {
        match self {
            DoorBodyType::SingleSwing { .. }
            | DoorBodyType::SingleSliding { .. }
            | DoorBodyType::Model { .. } => {
                vec!["body"]
            }
            DoorBodyType::DoubleSwing { .. } | DoorBodyType::DoubleSliding { .. } => {
                vec!["left", "right"]
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct DoorSegments {
    pub body: DoorBodyType,
    pub cue_inner: Entity,
    pub cue_outline: Entity,
}

fn make_door_visuals(
    entity: Entity,
    edge: &Edge,
    anchors: &AnchorParams,
    kind: &DoorType,
) -> (Transform, Vec<Transform>, Mesh, Mesh) {
    let p_start = anchors
        .point_in_parent_frame_of(edge.left(), Category::Door, entity)
        .unwrap();
    let p_end = anchors
        .point_in_parent_frame_of(edge.right(), Category::Door, entity)
        .unwrap();

    let dp = p_start - p_end;
    let length = dp.length();
    let yaw = (-dp.x).atan2(dp.y);
    let center = (p_start + p_end) / 2.0;

    let (inner, outline) = make_door_cues(length, kind);

    let get_double_door_tfs = |mid_offset: f32| -> Vec<Transform> {
        let left_door_length = (length - DOUBLE_DOOR_GAP) / 2.0 - mid_offset;
        let right_door_length = (length - DOUBLE_DOOR_GAP) / 2.0 + mid_offset;
        vec![
            Transform {
                translation: Vec3::new(
                    0.,
                    (length + DOUBLE_DOOR_GAP) / 4.0 + mid_offset / 2.0,
                    DEFAULT_LEVEL_HEIGHT / 2.0,
                ),
                scale: Vec3::new(
                    DEFAULT_DOOR_THICKNESS,
                    left_door_length,
                    DEFAULT_LEVEL_HEIGHT,
                ),
                ..default()
            },
            Transform {
                translation: Vec3::new(
                    0.,
                    -(length + DOUBLE_DOOR_GAP) / 4.0 + mid_offset / 2.0,
                    DEFAULT_LEVEL_HEIGHT / 2.0,
                ),
                scale: Vec3::new(
                    DEFAULT_DOOR_THICKNESS,
                    right_door_length,
                    DEFAULT_LEVEL_HEIGHT,
                ),
                ..default()
            },
        ]
    };

    let door_tfs = match kind {
        // TODO(luca) implement model variant
        DoorType::SingleSwing(_) | DoorType::SingleSliding(_) | DoorType::Model(_) => {
            vec![Transform {
                translation: Vec3::new(0., 0., DEFAULT_LEVEL_HEIGHT / 2.0),
                scale: Vec3::new(DEFAULT_DOOR_THICKNESS, length, DEFAULT_LEVEL_HEIGHT),
                ..default()
            }]
        }
        DoorType::DoubleSwing(door) => get_double_door_tfs(door.compute_offset(length)),
        DoorType::DoubleSliding(door) => get_double_door_tfs(door.compute_offset(length)),
    };
    (
        Transform {
            translation: Vec3::new(center.x, center.y, 0.),
            rotation: Quat::from_rotation_z(yaw),
            ..default()
        },
        door_tfs,
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

fn door_swing_arc(
    door_width: f32,
    door_count: u32,
    offset: f32,
    pivot_on: Side,
    swing: Swing,
) -> MeshBuffer {
    let pivot = pivot_on.sign() * door_width / 2.0;
    let pivot = Vec3::new(0.0, pivot, DOOR_CUE_HEIGHT);
    let door_width = door_width / door_count as f32 + offset;
    let (initial_angle, sweep) = swing.swing_on_pivot(pivot_on);

    let initial_angle = Radians(match initial_angle {
        misc::Angle::Deg(n) => n.to_radians(),
        misc::Angle::Rad(n) => n,
    });
    let sweep = Radians(match sweep {
        misc::Angle::Deg(n) => n.to_radians(),
        misc::Angle::Rad(n) => n,
    });
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
            door_swing_arc(door_width, 1, 0.0, door.pivot_on, door.swing).into_mesh_and_outline()
        }
        DoorType::DoubleSwing(door) => {
            let mid = door.compute_offset(door_width);
            door_swing_arc(door_width, 2, -mid, Side::Left, door.swing)
                .merge_with(door_swing_arc(door_width, 2, mid, Side::Right, door.swing))
                .into_mesh_and_outline()
        }
        _ => {
            let mut mesh = Mesh::new(
                PrimitiveTopology::TriangleList,
                RenderAssetUsages::default(),
            );
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
            mesh.insert_indices(Indices::U32(vec![]));
            (mesh.clone(), mesh)
        }
    }
}

pub fn add_door_visuals(
    mut commands: Commands,
    new_doors: Query<
        (Entity, &Edge, &DoorType, Option<&Visibility>),
        (
            Or<(Added<DoorType>, Added<Edge>)>,
            Without<DoorSegments>,
        ),
    >,
    anchors: AnchorParams,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, edge, kind, visibility) in &new_doors {
        let (pose_tf, door_tfs, cue_inner_mesh, cue_outline_mesh) =
            make_door_visuals(e, edge, &anchors, kind);

        let bodies = door_tfs
            .iter()
            .map(|tf| {
                commands
                    .spawn((
                        Mesh3d(assets.box_mesh.clone()),
                        MeshMaterial3d(assets.door_body_material.clone()),
                        *tf,
                        Visibility::default(),
                    ))
                    .insert(Selectable::new(e))
                    .id()
            })
            .collect::<Vec<_>>();
        let body = DoorBodyType::from_door_type(kind, &bodies);
        let cue_inner = commands
            .spawn((
                Mesh3d(meshes.add(cue_inner_mesh)),
                MeshMaterial3d(assets.translucent_white.clone()),
                Transform::default(),
                Visibility::default(),
            ))
            .id();

        let cue_outline = commands
            .spawn((
                Mesh3d(meshes.add(cue_outline_mesh)),
                MeshMaterial3d(assets.translucent_black.clone()),
                Transform::default(),
                Visibility::default(),
            ))
            .id();

        // Level doors for lifts may have already been given a Visibility
        // component upon creation, in which case we should respect whatever
        // value was set for it.
        let visibility = visibility.cloned().unwrap_or(Visibility::Inherited);

        commands
            .entity(e)
            .insert((pose_tf, visibility))
            .insert(DoorSegments {
                body,
                cue_inner,
                cue_outline,
            })
            .insert(Category::Door)
            .insert(EdgeLabels::LeftRight)
            .add_children(&[cue_inner, cue_outline])
            .add_children(&bodies);

        for anchor in edge.array() {
            if let Ok(mut deps) = dependents.get_mut(anchor) {
                deps.insert(e);
            }
        }
    }
}

fn update_door_visuals(
    commands: &mut Commands,
    entity: Entity,
    edge: &Edge,
    kind: &DoorType,
    segments: &DoorSegments,
    anchors: &AnchorParams,
    transforms: &mut Query<&mut Transform>,
    mesh_handles: &mut Query<&mut Mesh3d>,
    mesh_assets: &mut ResMut<Assets<Mesh>>,
    assets: &Res<SiteAssets>,
) -> Option<DoorBodyType> {
    let (pose_tf, door_tfs, cue_inner_mesh, cue_outline_mesh) =
        make_door_visuals(entity, edge, anchors, kind);
    let mut door_transform = transforms.get_mut(entity).unwrap();
    *door_transform = pose_tf;
    let mut entities = segments.body.entities();
    for (door_tf, e) in door_tfs.iter().zip(entities.iter()) {
        let mut door_transform = transforms.get_mut(*e).unwrap();
        *door_transform = *door_tf;
    }
    for door_tf in door_tfs.iter().skip(entities.len()) {
        // New doors were added, we need to spawn them
        let id = commands
            .spawn((
                Mesh3d(assets.box_mesh.clone()),
                MeshMaterial3d(assets.door_body_material.clone()),
                *door_tf,
                Visibility::default(),
            ))
            .insert(Selectable::new(entity))
            .id();
        entities.push(id);
        commands.entity(entity).add_child(id);
    }
    for e in entities.iter().skip(door_tfs.len()) {
        // Doors were removed, we need to despawn them
        commands.entity(*e).despawn();
    }
    let mut cue_inner = mesh_handles.get_mut(segments.cue_inner).unwrap();
    *cue_inner = Mesh3d(mesh_assets.add(cue_inner_mesh));
    let mut cue_outline = mesh_handles.get_mut(segments.cue_outline).unwrap();
    *cue_outline = Mesh3d(mesh_assets.add(cue_outline_mesh));
    let new_segments = DoorBodyType::from_door_type(kind, &entities);
    if new_segments != segments.body {
        Some(new_segments)
    } else {
        None
    }
}

pub fn update_changed_door(
    mut commands: Commands,
    mut doors: Query<
        (
            Entity,
            &Edge,
            &DoorType,
            &mut DoorSegments,
            &mut Hovered,
        ),
        Or<(Changed<Edge>, Changed<DoorType>)>,
    >,
    anchors: AnchorParams,
    mut transforms: Query<&mut Transform>,
    mut mesh_handles: Query<&mut Mesh3d>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    assets: Res<SiteAssets>,
) {
    for (entity, edge, kind, mut segments, mut hovered) in &mut doors {
        let old_door_count = segments.body.entities().len();
        if let Some(new_body) = update_door_visuals(
            &mut commands,
            entity,
            edge,
            kind,
            &segments,
            &anchors,
            &mut transforms,
            &mut mesh_handles,
            &mut mesh_assets,
            &assets,
        ) {
            segments.body = new_body;
            if segments.body.entities().len() > old_door_count {
                // A new door was spawned, trigger hovered change detection to update the outline
                // for the new mesh
                hovered.set_changed();
            }
        }
    }
}

pub fn update_door_for_moved_anchors(
    mut commands: Commands,
    mut doors: Query<(Entity, &Edge, &DoorType, &DoorSegments)>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    mut transforms: Query<&mut Transform>,
    mut mesh_handles: Query<&mut Mesh3d>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    assets: Res<SiteAssets>,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Some((entity, edge, kind, segments)) = doors.get_mut(*dependent).ok() {
                update_door_visuals(
                    &mut commands,
                    entity,
                    edge,
                    kind,
                    &segments,
                    &anchors,
                    &mut transforms,
                    &mut mesh_handles,
                    &mut mesh_assets,
                    &assets,
                );
            }
        }
    }
}

/// Unique UUID to identify issue of duplicated door names
pub const DUPLICATED_DOOR_NAME_ISSUE_UUID: Uuid =
    Uuid::from_u128(0x73f641f2a08d4ffd90216eb9bacb4743u128);

// When triggered by a validation request event, check if there are duplicated door names and
// generate an issue if that is the case
pub fn check_for_duplicated_door_names(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    child_of: Query<&ChildOf>,
    door_names: Query<(Entity, &NameInSite), With<DoorMarker>>,
) {
    for root in validate_events.read() {
        let mut names: HashMap<String, BTreeSet<Entity>> = HashMap::new();
        for (e, name) in &door_names {
            if AncestorIter::new(&child_of, e).any(|p| p == **root) {
                let entities_with_name = names.entry(name.0.clone()).or_default();
                entities_with_name.insert(e);
            }
        }
        for (name, entities) in names.drain() {
            if entities.len() > 1 {
                let issue = Issue {
                    key: IssueKey {
                        entities: entities,
                        kind: DUPLICATED_DOOR_NAME_ISSUE_UUID,
                    },
                    brief: format!("Multiple doors found with the same name {}", name),
                    hint: "Doors use their names as identifiers with RMF and each door should have a unique \
                           name, rename the affected doors".to_string()
                };
                let id = commands.spawn(issue).id();
                commands.entity(**root).add_child(id);
            }
        }
    }
}
