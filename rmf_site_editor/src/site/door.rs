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
    interaction::{PreviewableMarker, Selectable, SpawnPreview},
    shapes::*,
    site::*,
};
use bevy::{
    prelude::*,
    render::mesh::{Indices, PrimitiveTopology},
};
use itertools::Itertools;
use rmf_site_format::{Category, DoorType, Edge, DEFAULT_LEVEL_HEIGHT};

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
    pub fn from_door_type(door_type: &DoorType, entities: &[Entity]) -> Self {
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
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum DoorState {
    Open,
    Closed,
    Moving,
}

#[derive(Component, Debug, Clone, Copy, PartialEq)]
pub enum DoorCommand {
    Open,
    Close,
}

impl DoorCommand {
    pub fn to_state(self) -> DoorState {
        match self {
            DoorCommand::Open => DoorState::Open,
            DoorCommand::Close => DoorState::Closed,
        }
    }
}

#[derive(Debug, Clone, Copy, Component)]
pub struct DoorSegments {
    pub body: DoorBodyType,
    pub cue_inner: Entity,
    pub cue_outline: Entity,
}

fn get_double_door_tfs(double_door_width: f32, mid_offset: f32) -> Vec<Transform> {
    let left_door_length = (double_door_width - DOUBLE_DOOR_GAP) / 2.0 - mid_offset;
    let right_door_length = (double_door_width - DOUBLE_DOOR_GAP) / 2.0 + mid_offset;
    vec![
        Transform {
            translation: Vec3::new(
                0.,
                (double_door_width + DOUBLE_DOOR_GAP) / 4.0 + mid_offset / 2.0,
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
                -(double_door_width + DOUBLE_DOOR_GAP) / 4.0 + mid_offset / 2.0,
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
}

fn make_door_visuals(
    entity: Entity,
    edge: &Edge<Entity>,
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

    let door_tfs = match kind {
        DoorType::SingleSwing(_) | DoorType::SingleSliding(_) | DoorType::Model(_) => {
            vec![Transform {
                translation: Vec3::new(0., 0., DEFAULT_LEVEL_HEIGHT / 2.0),
                scale: Vec3::new(DEFAULT_DOOR_THICKNESS, length, DEFAULT_LEVEL_HEIGHT),
                ..default()
            }]
        }
        DoorType::DoubleSwing(_) | DoorType::DoubleSliding(_) => {
            get_double_door_tfs(length, kind.compute_offset(length).unwrap())
        }
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
        DoorType::DoubleSliding(_) => {
            let left = (door_width - DOOR_STOP_LINE_THICKNESS) / 2.0;
            let mid = kind.compute_offset(door_width).unwrap();
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
            let mid = kind.compute_offset(door_width).unwrap();
            door_swing_arc(door_width, 2, -mid, Side::Left, door.swing)
                .merge_with(door_swing_arc(door_width, 2, mid, Side::Right, door.swing))
                .into_mesh_and_outline()
        }
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
    new_doors: Query<
        (Entity, &Edge<Entity>, &DoorType, Option<&Visibility>),
        (
            Or<(Added<DoorType>, Added<Edge<Entity>>)>,
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

        let mut commands = commands.entity(e);
        let (body, cue_inner, cue_outline) = commands.add_children(|parent| {
            let bodies = door_tfs
                .iter()
                .map(|tf| {
                    parent
                        .spawn(PbrBundle {
                            mesh: assets.box_mesh.clone(),
                            material: assets.door_body_material.clone(),
                            transform: *tf,
                            ..default()
                        })
                        .insert(Selectable::new(e))
                        .id()
                })
                .collect::<Vec<_>>();
            let body = DoorBodyType::from_door_type(kind, &bodies);

            let cue_inner = parent
                .spawn(PbrBundle {
                    mesh: meshes.add(cue_inner_mesh),
                    material: assets.translucent_white.clone(),
                    ..default()
                })
                .id();

            let cue_outline = parent
                .spawn(PbrBundle {
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
            .insert(SpatialBundle {
                transform: pose_tf,
                visibility: Visibility { is_visible },
                ..default()
            })
            .insert(DoorSegments {
                body,
                cue_inner,
                cue_outline,
            })
            .insert(Category::Door)
            .insert(DoorState::Open)
            .insert(DoorCommand::Open)
            .insert(EdgeLabels::LeftRight);

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
    edge: &Edge<Entity>,
    kind: &DoorType,
    segments: &DoorSegments,
    anchors: &AnchorParams,
    transforms: &mut Query<&mut Transform>,
    mesh_handles: &mut Query<&mut Handle<Mesh>>,
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
            .spawn(PbrBundle {
                mesh: assets.box_mesh.clone(),
                material: assets.door_body_material.clone(),
                transform: *door_tf,
                ..default()
            })
            .insert(Selectable::new(entity))
            .id();
        entities.push(id);
        commands.entity(entity).add_child(id);
    }
    for e in entities.iter().skip(door_tfs.len()) {
        // Doors were removed, we need to despawn them
        commands.entity(*e).despawn_recursive();
    }
    let mut cue_inner = mesh_handles.get_mut(segments.cue_inner).unwrap();
    *cue_inner = mesh_assets.add(cue_inner_mesh);
    let mut cue_outline = mesh_handles.get_mut(segments.cue_outline).unwrap();
    *cue_outline = mesh_assets.add(cue_outline_mesh);
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
        (Entity, &Edge<Entity>, &DoorType, &mut DoorSegments),
        Or<(Changed<Edge<Entity>>, Changed<DoorType>)>,
    >,
    anchors: AnchorParams,
    mut transforms: Query<&mut Transform>,
    mut mesh_handles: Query<&mut Handle<Mesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    assets: Res<SiteAssets>,
) {
    for (entity, edge, kind, mut segments) in &mut doors {
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
        }
    }
}

pub fn update_door_for_moved_anchors(
    mut commands: Commands,
    doors: Query<(Entity, &Edge<Entity>, &DoorType, &DoorSegments)>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    mut transforms: Query<&mut Transform>,
    mut mesh_handles: Query<&mut Handle<Mesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    assets: Res<SiteAssets>,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Ok((entity, edge, kind, segments)) = doors.get(*dependent) {
                update_door_visuals(
                    &mut commands,
                    entity,
                    edge,
                    kind,
                    segments,
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

pub fn manage_door_previews(
    mut commands: Commands,
    mut preview_events: EventReader<SpawnPreview>,
    previewable_doors: Query<(&DoorState, Option<&DoorCommand>), With<PreviewableMarker>>,
) {
    for event in preview_events.iter() {
        match event.entity {
            None => {
                // TODO(luca) stop the door preview
            }
            Some(e) => {
                if let Ok((state, door_command)) = previewable_doors.get(e) {
                    let desired_state = match state {
                        DoorState::Closed => DoorCommand::Open,
                        DoorState::Open => DoorCommand::Close,
                        DoorState::Moving => {
                            *door_command.expect("Door is moving but no command was issued")
                        }
                    };
                    // TODO(luca) Check if insertion has performance implications and we should
                    // edit in place
                    commands.entity(e).insert(desired_state);
                }
            }
        }
    }
}

fn door_edge_length(entity: Entity, edge: &Edge<Entity>, anchors: &AnchorParams) -> f32 {
    let p_start = anchors
        .point_in_parent_frame_of(edge.left(), Category::Door, entity)
        .unwrap();
    let p_end = anchors
        .point_in_parent_frame_of(edge.right(), Category::Door, entity)
        .unwrap();

    let dp = p_start - p_end;
    dp.length()
}

fn door_closed_position(
    entity: Entity,
    edge: &Edge<Entity>,
    kind: &DoorType,
    body: &DoorBodyType,
    anchors: &AnchorParams,
) -> Vec<Transform> {
    match body {
        DoorBodyType::SingleSwing { .. }
        | DoorBodyType::SingleSliding { .. }
        | DoorBodyType::Model { .. } => {
            vec![Transform {
                translation: Vec3::new(0., 0., DEFAULT_LEVEL_HEIGHT / 2.0),
                ..default()
            }]
        }
        DoorBodyType::DoubleSwing { .. } | DoorBodyType::DoubleSliding { .. } => {
            let length = door_edge_length(entity, edge, anchors);
            let mid_offset = kind.compute_offset(length).expect("Mismatch");
            get_double_door_tfs(length, mid_offset)
        }
    }
}

// TODO(luca) If we were careful about system ordering this system could be made to return a vec
// and panic on failure instead. It will currently only fail for off-by-one-frame system ordering
// issues that cause mismatches between DoorType and DoorBodyType
fn door_open_position(
    entity: Entity,
    edge: &Edge<Entity>,
    kind: &DoorType,
    body: &DoorBodyType,
    transforms: &[&Transform],
    anchors: &AnchorParams,
) -> Option<Vec<Transform>> {
    fn swing_angle(swing: &Swing) -> f32 {
        match swing {
            Swing::Forward(angle) => angle.radians(),
            Swing::Backward(angle) => -angle.radians(),
            Swing::Both { forward, .. } => forward.radians(),
        }
    }
    match body {
        DoorBodyType::SingleSwing { .. } => {
            let tf = transforms.get(0)?;
            let kind = kind.single_swing()?;
            let open_position = swing_angle(&kind.swing);
            Some(vec![Transform {
                translation: Vec3::new(
                    (tf.scale.y / 2.0) * open_position.sin(),
                    (tf.scale.y / 2.0) * (1.0 - open_position.cos()) * kind.pivot_on.sign(),
                    DEFAULT_LEVEL_HEIGHT / 2.0,
                ),
                rotation: Quat::from_rotation_z(open_position * kind.pivot_on.sign()),
                ..default()
            }])
        }
        DoorBodyType::DoubleSwing { .. } => {
            let double_swing = kind.double_swing()?;
            let open_position = swing_angle(&double_swing.swing);
            let length = door_edge_length(entity, edge, anchors);
            let mid_offset = kind.compute_offset(length)?;
            let tfs = get_double_door_tfs(length, mid_offset);
            let (left_tf, right_tf) = tfs.iter().collect_tuple()?;
            Some(vec![
                Transform {
                    translation: Vec3::new(
                        (left_tf.scale.y / 2.0) * open_position.sin(),
                        left_tf.translation.y
                            + (left_tf.scale.y / 2.0) * (1.0 - open_position.cos()),
                        DEFAULT_LEVEL_HEIGHT / 2.0,
                    ),
                    rotation: Quat::from_rotation_z(open_position),
                    ..default()
                },
                Transform {
                    translation: Vec3::new(
                        (right_tf.scale.y / 2.0) * open_position.sin(),
                        right_tf.translation.y
                            - (right_tf.scale.y / 2.0) * (1.0 - open_position.cos()),
                        DEFAULT_LEVEL_HEIGHT / 2.0,
                    ),
                    rotation: Quat::from_rotation_z(-open_position),
                    ..default()
                },
            ])
        }
        DoorBodyType::SingleSliding { .. } => {
            let tf = transforms.get(0)?;
            let kind = kind.single_sliding()?;
            Some(vec![Transform {
                translation: Vec3::new(
                    0.,
                    tf.scale.y * kind.towards.sign(),
                    DEFAULT_LEVEL_HEIGHT / 2.0,
                ),
                ..default()
            }])
        }
        DoorBodyType::DoubleSliding { .. } => {
            let length = door_edge_length(entity, edge, anchors);
            let mid_offset = kind.compute_offset(length)?;
            let tfs = get_double_door_tfs(length, mid_offset);
            let (left_tf, right_tf) = tfs.iter().collect_tuple()?;
            Some(vec![
                Transform {
                    translation: Vec3::new(
                        0.,
                        left_tf.translation.y + left_tf.scale.y,
                        DEFAULT_LEVEL_HEIGHT / 2.0,
                    ),
                    ..default()
                },
                Transform {
                    translation: Vec3::new(
                        0.,
                        right_tf.translation.y - right_tf.scale.y,
                        DEFAULT_LEVEL_HEIGHT / 2.0,
                    ),
                    ..default()
                },
            ])
        }
        DoorBodyType::Model { .. } => {
            warn!("Model open position not implemented");
            None
        }
    }
}

pub fn control_doors(
    door_commands: Query<(
        Entity,
        &DoorCommand,
        &DoorType,
        &DoorState,
        &DoorSegments,
        &Edge<Entity>,
    )>,
    mut transforms: Query<&mut Transform>,
    anchors: AnchorParams,
) {
    for (entity, cmd, kind, state, segments, edge) in &door_commands {
        if cmd.to_state() != *state {
            let segment_tfs = segments
                .body
                .entities()
                .iter()
                .map(|e| {
                    transforms
                        .get(*e)
                        .expect("Transform for door body not found")
                })
                .collect::<Vec<_>>();
            let target_positions = match cmd {
                DoorCommand::Open => {
                    let Some(val) = door_open_position(
                        entity,
                        edge,
                        kind,
                        &segments.body,
                        &segment_tfs,
                        &anchors) else {
                        continue;
                    };
                    val
                }
                DoorCommand::Close => {
                    door_closed_position(entity, edge, kind, &segments.body, &anchors)
                }
            };
            for (e, target_tf) in segments.body.entities().iter().zip(target_positions.iter()) {
                let mut tf = transforms.get_mut(*e).unwrap();
                tf.translation = target_tf.translation;
                tf.rotation = target_tf.rotation;
            }
        }
    }
}

pub fn update_door_state(
    mut doors: Query<(
        Entity,
        &DoorType,
        &mut DoorState,
        &DoorSegments,
        &Edge<Entity>,
    )>,
    transforms: Query<&Transform>,
    anchors: AnchorParams,
) {
    fn transforms_approx_equal(tf1: &Transform, tf2: &Transform) -> bool {
        tf1.rotation.angle_between(tf2.rotation).abs() < 1e-3
            && tf1.translation.distance(tf2.translation) < 1e-3
    }
    for (e, kind, mut state, segments, edge) in &mut doors {
        let segment_tfs = segments
            .body
            .entities()
            .iter()
            .map(|e| {
                transforms
                    .get(*e)
                    .expect("Transform for door body not found")
            })
            .collect::<Vec<_>>();
        let Some(open_tfs) = door_open_position(e, edge, kind, &segments.body, &segment_tfs, &anchors) else {
            continue;
        };
        let closed_tfs = door_closed_position(e, edge, kind, &segments.body, &anchors);
        let mut all_open = true;
        let mut all_closed = true;
        for (segment_tf, open_tf, closed_tf) in
            itertools::izip!(&segment_tfs, &open_tfs, &closed_tfs)
        {
            if !transforms_approx_equal(segment_tf, open_tf) {
                all_open = false;
            } else if !transforms_approx_equal(segment_tf, closed_tf) {
                all_closed = false;
            }
        }
        if all_open {
            *state = DoorState::Open;
        } else if all_closed {
            *state = DoorState::Closed;
        } else {
            *state = DoorState::Moving;
        }
    }
}
