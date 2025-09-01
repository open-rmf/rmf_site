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

use crate::{issue::*, site::*, layers::ZLayer};
use bevy::{
    ecs::{hierarchy::ChildOf, relationship::AncestorIter},
    prelude::*,
    render::{
        mesh::{Indices, PrimitiveTopology},
        render_asset::RenderAssetUsages,
    },
};
use bevy_rich_text3d::*;
use rmf_site_format::{Category, DoorType, Edge, DEFAULT_LEVEL_HEIGHT};
use rmf_site_mesh::{
    flat_arc, flat_arrow_mesh_between, line_stroke_away_from, line_stroke_mesh, MeshBuffer, Radians,
};
use rmf_site_picking::{Hovered, Selectable, VisualCue};
use std::collections::{BTreeSet, HashMap};
use std::num::NonZero;
use uuid::Uuid;

pub const DOOR_CUE_HEIGHT: f32 = 0.004;
pub const DOOR_STOP_LINE_THICKNESS: f32 = 0.01;
pub const DOOR_STOP_LINE_LENGTH: f32 = 3.0 * DEFAULT_DOOR_THICKNESS;
pub const DOOR_SWEEP_THICKNESS: f32 = 0.05;
pub const DOUBLE_DOOR_GAP: f32 = 0.02;
const DOOR_NAME_LINE_LIMIT: usize = 30;
const DOOR_NAME_CHARACTER_LENGTH: f32 = 0.07;

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
    pub name_door: Entity,
    pub name_floor: Entity,
}

fn find_door_character_limit(door_length: f32) -> usize {
    // 4.0 is subtracted to account for side margins.
    let number_of_characters = door_length / DOOR_NAME_CHARACTER_LENGTH - 4.0;
    // Maximum limit is DOOR_NAME_LINE_LIMIT, minimum limit is 1.
    let new_limit = (1).max((number_of_characters.floor() as usize).min(DOOR_NAME_LINE_LIMIT));
    new_limit
}

fn handle_name_limit(limit: usize, name: &str) -> String {
    if name.len() <= limit || limit == 0 {
        return name.to_string();
    }
    let mut result = String::new();
    let mut current_line_length = 0;

    for c in name.chars() {
        if current_line_length >= limit {
            result.push('\n');
            current_line_length = 0;
        }
        result.push(c);
        current_line_length += 1;
    }
    result
}

fn find_door_position_tfs(kind: &DoorType, length: f32, offset: f32) -> Vec<Transform> {
    let door_slide_tf = |side: Side, position, door_length, is_double, offset| {
        let (translation_offset, gap) = match is_double {
            false => (0.0, 0.0),
            true => (door_length * 0.5, DOUBLE_DOOR_GAP / 2.0),
        };
        let distance = (door_length * position + gap + translation_offset) * side.sign();

        Transform {
            translation: Vec3::new(0., distance + offset, DEFAULT_LEVEL_HEIGHT / 2.0),
            scale: Vec3::new(DEFAULT_DOOR_THICKNESS, door_length, DEFAULT_LEVEL_HEIGHT),
            ..default()
        }
    };

    let door_swing_tf = |side: Side, swing: Swing, position, door_length, is_double, offset| {
        let door_radius = door_length * 0.5 * side.sign();

        let (_, sweep) = swing.swing_on_pivot(side);
        let sweep = sweep.radians();

        let sweep: f32 = match swing {
            Swing::Both {
                forward: f,
                backward: b,
            } => {
                let f = f.radians();
                let b = b.radians();
                let angle = position * (f + b) - b;
                angle * side.sign()
            }
            _ => position * sweep,
        };

        let new_x = door_radius * sweep.sin();
        let new_y = match is_double {
            false => door_radius - door_radius * sweep.cos(),
            true => {
                let gap = side.sign() * DOUBLE_DOOR_GAP / 2.0;
                door_length * side.sign() - door_radius * sweep.cos() + gap
            }
        };

        let door_swing_rotation = Quat::from_axis_angle(Vec3::Z, sweep);

        Transform {
            translation: Vec3::new(new_x, new_y + offset, DEFAULT_LEVEL_HEIGHT / 2.0),
            scale: Vec3::new(DEFAULT_DOOR_THICKNESS, door_length, DEFAULT_LEVEL_HEIGHT),
            rotation: door_swing_rotation,
        }
    };

    let left_door_length = (length - DOUBLE_DOOR_GAP) / 2.0 - offset;
    let right_door_length = (length - DOUBLE_DOOR_GAP) / 2.0 + offset;

    match kind {
        DoorType::SingleSliding(door) => vec![door_slide_tf(
            door.towards,
            door.position,
            length,
            false,
            0.0,
        )],
        DoorType::SingleSwing(door) => vec![door_swing_tf(
            door.pivot_on,
            door.swing,
            door.position,
            length,
            false,
            0.0,
        )],
        DoorType::DoubleSliding(door) => vec![
            door_slide_tf(
                Side::Left,
                door.left_position,
                left_door_length,
                true,
                offset,
            ),
            door_slide_tf(
                Side::Right,
                door.right_position,
                right_door_length,
                true,
                offset,
            ),
        ],
        DoorType::DoubleSwing(door) => vec![
            door_swing_tf(
                Side::Left,
                door.swing,
                door.left_position,
                left_door_length,
                true,
                offset,
            ),
            door_swing_tf(
                Side::Right,
                door.swing,
                door.right_position,
                right_door_length,
                true,
                offset,
            ),
        ],
        DoorType::Model(_) => vec![door_slide_tf(Side::Left, 0.0, length, false, 0.0)],
    }
}

fn make_door_visuals(
    entity: Entity,
    edge: &Edge<Entity>,
    anchors: &AnchorParams,
    kind: &DoorType,
) -> (Transform, Vec<Transform>, Mesh, Mesh, f32, Vec3) {
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
        // TODO(luca) implement model variant
        DoorType::SingleSwing(_) | DoorType::SingleSliding(_) | DoorType::Model(_) => {
            find_door_position_tfs(kind, length, 0.0)
        }
        DoorType::DoubleSwing(door) => {
            find_door_position_tfs(kind, length, door.compute_offset(length))
        }
        DoorType::DoubleSliding(door) => {
            find_door_position_tfs(kind, length, door.compute_offset(length))
        }
    };

    let name_scale = Vec3::new(1.0 / door_tfs[0].scale.z, 1.0 / door_tfs[0].scale.y, 1.0);

    (
        Transform {
            translation: Vec3::new(center.x, center.y, 0.),
            rotation: Quat::from_rotation_z(yaw),
            ..default()
        },
        door_tfs,
        inner,
        outline,
        length,
        name_scale,
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

    let arc_length = std::f32::consts::PI * door_width;
    let texture_thickness = DOOR_STOP_LINE_THICKNESS / DOOR_SWEEP_THICKNESS;

    flat_arc(
        pivot,
        door_width,
        DOOR_SWEEP_THICKNESS,
        initial_angle,
        sweep,
        0.5,
    )
    .scale_uv(arc_length, 1.0)
    .merge_with(
        line_stroke_away_from(
            pivot + pivot_on.sign() * DOOR_STOP_LINE_THICKNESS / 2.0 * Vec3::Y,
            initial_angle,
            door_width,
            DOOR_STOP_LINE_THICKNESS,
        )
        .scale_uv(door_width, texture_thickness),
    )
    .merge_with(
        line_stroke_away_from(
            pivot + pivot_on.sign() * DOOR_STOP_LINE_THICKNESS / 2.0 * Vec3::Y,
            initial_angle + sweep,
            door_width,
            DOOR_STOP_LINE_THICKNESS,
        )
        .scale_uv(door_width, texture_thickness),
    )
}

fn make_door_cues(door_width: f32, kind: &DoorType) -> (Mesh, Mesh) {
    match kind {
        DoorType::SingleSliding(door) => {
            let start =
                door.towards.opposite().sign() * (door_width - DOOR_STOP_LINE_THICKNESS) / 2.0;
            let stop = door.towards.sign() * (door_width - DOOR_STOP_LINE_THICKNESS) / 2.0;
            door_slide_stop_line(-door_width / 2.0)
                .merge_with(door_slide_stop_line(door_width / 2.0))
                .add_normalised_uv()
                .merge_with(
                    door_slide_arrows(start, stop)
                        .add_normalised_uv()
                        .flip_uv()
                        .scale_uv(door_width, 1.0),
                )
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
                .add_normalised_uv()
                .merge_with(
                    (door_slide_arrows(mid + tweak, left - tweak))
                        .merge_with(door_slide_arrows(mid - tweak, right + tweak))
                        .add_normalised_uv()
                        .flip_uv()
                        .scale_uv(door_width, 1.0),
                )
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

fn create_door_name(
    commands: &mut Commands,
    name: &NameInSite,
    scale: Vec3,
    assets: &Res<SiteAssets>,
    door_tf_length: f32,
    door_space_length: f32,
) -> (Entity, Entity) {
    let name_str = &name.0;

    let transform_door = (
        Transform {
            translation: Vec3::new(DEFAULT_DOOR_THICKNESS + 0.5, 0.0, 0.0),
            rotation: Quat::from_rotation_y(90_f32.to_radians()),
            scale: scale,
        },
        door_tf_length,
    );
    let transform_floor = (
        Transform {
            // TODO(@mxgrey): For some reason the text won't show over a hovered
            // layer unless we multiply this z value by 2, even though it should
            // win in the z-buffer without that. We should investigate the cause
            // of this.
            translation: Vec3::new(0.0, 0.0, 2.0*ZLayer::LabelText.to_z()),
            rotation: Quat::from_rotation_z(90_f32.to_radians()),
            scale: Vec3::ONE,
        },
        door_space_length,
    );

    let mut spawn_name = |(tf, length)| {
        let line_limit = find_door_character_limit(length);
        let name = if name_str.len() > line_limit {
            handle_name_limit(line_limit, name_str)
        } else {
            name_str.clone()
        };

        commands
            .spawn((
                Text3d::new(name),
                Text3dStyling {
                    size: 250.,
                    weight: Weight(900),
                    stroke: NonZero::new(15),
                    color: Srgba::BLACK,
                    stroke_color: Srgba::WHITE,
                    world_scale: Some(Vec2::splat(0.125)),
                    layer_offset: 0.001,
                    align: TextAlign::Center,
                    ..Default::default()
                },
                Mesh3d::default(),
                MeshMaterial3d(assets.text3d_material.clone()),
                tf,
            ))
            .id()
    };
    let name_on_door = spawn_name(transform_door);
    let name_on_floor = spawn_name(transform_floor);

    (name_on_door, name_on_floor)
}

pub fn add_door_visuals(
    mut commands: Commands,
    new_doors: Query<
        (
            Entity,
            &Edge<Entity>,
            &DoorType,
            &NameInSite,
            Option<&Visibility>,
        ),
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
    for (e, edge, kind, name, visibility) in &new_doors {
        let (pose_tf, door_tfs, cue_inner_mesh, cue_outline_mesh, door_length, name_scale) =
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
                MeshMaterial3d(assets.door_cue_material.clone()),
                Transform::default(),
                Visibility::default(),
            ))
            .insert(Selectable::new(e))
            .id();

        let cue_outline = commands
            .spawn((
                Mesh3d(meshes.add(cue_outline_mesh)),
                MeshMaterial3d(assets.translucent_black.clone()),
                Transform::default(),
                Visibility::default(),
            ))
            .id();

        let (parent_entity, parent_tf) = (bodies[0], door_tfs[0]);
        let (name_door, name_floor) = create_door_name(
            &mut commands,
            name,
            name_scale,
            &assets,
            parent_tf.scale.y,
            door_length,
        );

        commands.entity(parent_entity).add_child(name_door);
        commands.entity(name_door).insert(VisualCue::no_outline());
        commands.entity(name_floor).insert(VisualCue::no_outline());

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
                name_door,
                name_floor,
            })
            .insert(Category::Door)
            .insert(EdgeLabels::LeftRight)
            .add_children(&[cue_inner, cue_outline])
            .add_children(&bodies)
            .add_child(name_floor);

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
    name: &NameInSite,
    anchors: &AnchorParams,
    transforms: &mut Query<&mut Transform>,
    mesh_handles: &mut Query<&mut Mesh3d>,
    texts: &mut Query<&mut Text3d>,
    mesh_assets: &mut ResMut<Assets<Mesh>>,
    assets: &Res<SiteAssets>,
) -> Option<DoorBodyType> {
    let (pose_tf, door_tfs, cue_inner_mesh, cue_outline_mesh, door_length, child_scale) =
        make_door_visuals(entity, edge, anchors, kind);
    let mut door_transform = transforms.get_mut(entity).unwrap();
    *door_transform = pose_tf;
    let mut entities = segments.body.entities();
    for (door_tf, e) in door_tfs.iter().zip(entities.iter()) {
        let mut door_transform = transforms.get_mut(*e).unwrap();
        *door_transform = *door_tf;
    }
    update_door_name(
        door_length,
        name,
        segments.name_door,
        segments.name_floor,
        child_scale,
        texts,
        transforms,
    );

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

fn update_door_name(
    door_length: f32,
    name: &NameInSite,
    door_entity: Entity,
    floor_entity: Entity,
    name_scale: Vec3,
    texts: &mut Query<&mut Text3d>,
    transforms: &mut Query<&mut Transform>,
) {
    let Ok(mut name_door_tf) = transforms.get_mut(door_entity) else {
        return;
    };
    // Rescale name
    name_door_tf.scale = name_scale;

    // Resize names for chaged name or changed door length
    let mut update_name = |length, entity| {
        let line_limit = find_door_character_limit(length);
        let new_name = if name.0.len() > line_limit {
            handle_name_limit(line_limit, &name.0)
        } else {
            name.0.clone()
        };
        if let Ok(mut text) = texts.get_mut(entity) {
            let prev_str = match &text.segments[0].0 {
                Text3dSegment::String(s) => s,
                _ => return,
            };
            if *prev_str != new_name {
                text.segments[0].0 = Text3dSegment::String(new_name.clone());
            }
        }
    };
    update_name(DEFAULT_LEVEL_HEIGHT, door_entity);
    update_name(door_length, floor_entity);
}

pub fn update_changed_door(
    mut commands: Commands,
    mut doors: Query<
        (
            Entity,
            &Edge<Entity>,
            &DoorType,
            &NameInSite,
            &mut DoorSegments,
            &mut Hovered,
        ),
        Or<(
            Changed<Edge<Entity>>,
            Changed<DoorType>,
            Changed<NameInSite>,
        )>,
    >,
    anchors: AnchorParams,
    mut transforms: Query<&mut Transform>,
    mut mesh_handles: Query<&mut Mesh3d>,
    mut texts: Query<&mut Text3d>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
    assets: Res<SiteAssets>,
) {
    for (entity, edge, kind, name, mut segments, mut hovered) in &mut doors {
        let old_door_count = segments.body.entities().len();
        if let Some(new_body) = update_door_visuals(
            &mut commands,
            entity,
            edge,
            kind,
            &segments,
            name,
            &anchors,
            &mut transforms,
            &mut mesh_handles,
            &mut texts,
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
    mut doors: Query<(Entity, &Edge<Entity>, &DoorType, &DoorSegments, &NameInSite)>,
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
    mut texts: Query<&mut Text3d>,
    assets: Res<SiteAssets>,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Some((entity, edge, kind, segments, name)) = doors.get_mut(*dependent).ok() {
                update_door_visuals(
                    &mut commands,
                    entity,
                    edge,
                    kind,
                    &segments,
                    &name,
                    &anchors,
                    &mut transforms,
                    &mut mesh_handles,
                    &mut texts,
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
