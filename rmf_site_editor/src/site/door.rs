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
    site::*,
    shapes::*,
};
use bevy::{
    prelude::*,
    render::mesh::{PrimitiveTopology, Indices},
};
use rmf_site_format::{DoorMarker, DoorType, Edge, DEFAULT_LEVEL_HEIGHT};

pub const DEFAULT_DOOR_THICKNESS: f32 = 0.1;
pub const DOOR_CUE_THICKNESS: f32 = 0.01;
pub const DOOR_CUE_HEIGHT: f32 = 0.004;
pub const DOOR_STOP_LINE_CUE_LENGTH: f32 = 3.0 * DEFAULT_DOOR_THICKNESS;

#[derive(Debug, Clone, Copy, Component)]
pub struct DoorSegments {
    // TODO(MXG): When it's time to animate the doors we should replace this
    // with an enum for the different possible door types: Single/Double Swing/Sliding
    pub body: Entity,
    pub cues: Entity,
}

fn make_door_visuals(
    edge: &Edge<Entity>,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
    kind: &DoorType,
) -> (Transform, Transform, Mesh) {
    let start_anchor = anchors.get(edge.left()).unwrap();
    let end_anchor = anchors.get(edge.right()).unwrap();

    let p_start = start_anchor.translation();
    let p_end = end_anchor.translation();
    let dp = p_start - p_end;
    let length = dp.length();
    let yaw = (-dp.x).atan2(dp.y);
    let center = (p_start + p_end) / 2.0;

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
        make_door_cues(length, kind),
    )
}

fn door_slide_stop_line(
    y: f32,
) -> MeshBuffer {
    let x_span = DOOR_STOP_LINE_CUE_LENGTH;
    line_stroke_mesh(
        Vec3::new(-x_span, y, DOOR_CUE_HEIGHT),
        Vec3::new(x_span, y, DOOR_CUE_HEIGHT),
        DOOR_CUE_THICKNESS,
    )
}

fn door_slide_arrow(
    start: f32,
    stop: f32,
    sign: f32,
) -> MeshBuffer {
    let x_max = DOOR_STOP_LINE_CUE_LENGTH;
    let tip = DEFAULT_DOOR_THICKNESS;
    let handle_thickness = DEFAULT_DOOR_THICKNESS/3.0;
    flat_arrow_mesh_between(
        Vec3::new(sign * (x_max - 2.0/3.0*tip), start, DOOR_CUE_HEIGHT),
        Vec3::new(sign * (x_max - 2.0/3.0*tip), stop, DOOR_CUE_HEIGHT),
        handle_thickness,
        tip,
        tip,
    )
}

fn door_slide_arrows(
    start: f32,
    stop: f32,
) -> MeshBuffer {
    door_slide_arrow(start, stop, -1.0)
    .merge_with(door_slide_arrow(start, stop, 1.0))
}

fn make_door_cues(
    door_width: f32,
    kind: &DoorType,
) -> Mesh {
    match kind {
        DoorType::SingleSliding(door) => {
            let start = door.towards.opposite().sign() * (door_width - DOOR_CUE_THICKNESS)/2.0;
            let stop = door.towards.sign() * (door_width - DOOR_CUE_THICKNESS)/2.0;
            let mut mesh: Mesh = door_slide_stop_line(-door_width/2.0).into();
            door_slide_stop_line(door_width/2.0).merge_into(&mut mesh);
            door_slide_arrows(start, stop).merge_into(&mut mesh);
            mesh
        },
        DoorType::DoubleSliding(door) => {
            let left = -(door_width - DOOR_CUE_THICKNESS)/2.0;
            let mid = door.compute_offset(door_width);
            let right = (door_width - DOOR_CUE_THICKNESS)/2.0;
            let tweak = DOOR_CUE_THICKNESS/2.0;
            let mut mesh: Mesh = door_slide_stop_line(left).into();
            door_slide_stop_line(mid).merge_into(&mut mesh);
            door_slide_stop_line(right).merge_into(&mut mesh);
            door_slide_arrows(mid-tweak, left+tweak).merge_into(&mut mesh);
            door_slide_arrows(mid+tweak, right-tweak).merge_into(&mut mesh);
            mesh
        }
        _ => {
            let mut mesh = Mesh::new(PrimitiveTopology::TriangleList);
            mesh.insert_attribute(Mesh::ATTRIBUTE_POSITION, Vec::<[f32; 3]>::new());
            mesh.insert_attribute(Mesh::ATTRIBUTE_NORMAL, Vec::<[f32; 3]>::new());
            mesh.set_indices(Some(Indices::U32(vec![])));
            mesh
        }
    }
}

pub fn add_door_visuals(
    mut commands: Commands,
    new_doors: Query<(Entity, &Edge<Entity>, &DoorType), Added<DoorMarker>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut dependents: Query<&mut AnchorDependents>,
    assets: Res<SiteAssets>,
    mut meshes: ResMut<Assets<Mesh>>,
) {
    for (e, edge, kind) in &new_doors {
        let (pose_tf, shape_tf, cues_mesh) = make_door_visuals(edge, &anchors, kind);

        let mut commands = commands.entity(e);
        let (body, cues) = commands.add_children(|parent| {
            let body = parent
                .spawn_bundle(PbrBundle {
                    mesh: assets.box_mesh.clone(),
                    material: assets.door_body_material.clone(),
                    transform: shape_tf,
                    ..default()
                })
                .insert(Selectable::new(e))
                .id();

            let cues = parent
                .spawn_bundle(PbrBundle {
                    mesh: meshes.add(cues_mesh),
                    material: assets.door_cue_material.clone(),
                    ..default()
                }).id();

            (body, cues)
        });

        commands
            .insert_bundle(SpatialBundle {
                transform: pose_tf,
                ..default()
            })
            .insert(DoorSegments { body, cues })
            .insert(Category("Door".to_string()))
            .insert(EdgeLabels::LeftRight);

        for anchor in &edge.array() {
            if let Ok(mut dep) = dependents.get_mut(*anchor) {
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
    anchors: &Query<&GlobalTransform, With<Anchor>>,
    transforms: &mut Query<&mut Transform>,
    mesh_handles: &mut Query<&mut Handle<Mesh>>,
    mesh_assets: &mut ResMut<Assets<Mesh>>,
) {
    let (pose_tf, shape_tf, cues_mesh) = make_door_visuals(edge, anchors, kind);
    let mut door_transform = transforms.get_mut(entity).unwrap();
    *door_transform = pose_tf;
    let mut shape_transform = transforms.get_mut(segments.body).unwrap();
    *shape_transform = shape_tf;
    let mut cues = mesh_handles.get_mut(segments.cues).unwrap();
    *cues = mesh_assets.add(cues_mesh);
}

pub fn update_changed_door(
    doors: Query<(Entity, &Edge<Entity>, &DoorType, &DoorSegments), Or<(Changed<Edge<Entity>>, Changed<DoorType>)>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut transforms: Query<&mut Transform>,
    mut mesh_handles: Query<&mut Handle<Mesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
) {
    for (entity, edge, kind, segments) in &doors {
        update_door_visuals(
            entity, edge, kind, segments, &anchors,
            &mut transforms, &mut mesh_handles, &mut mesh_assets
        );
    }
}

pub fn update_door_for_changed_anchor(
    doors: Query<(Entity, &Edge<Entity>, &DoorType, &DoorSegments), With<DoorMarker>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    changed_anchors: Query<&AnchorDependents, (With<Anchor>, Changed<GlobalTransform>)>,
    mut transforms: Query<&mut Transform>,
    mut mesh_handles: Query<&mut Handle<Mesh>>,
    mut mesh_assets: ResMut<Assets<Mesh>>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((entity, edge, kind, segments)) = doors.get(*dependent).ok() {
                update_door_visuals(
                    entity, edge, kind, segments, &anchors,
                    &mut transforms, &mut mesh_handles, &mut mesh_assets
                );
            }
        }
    }
}
