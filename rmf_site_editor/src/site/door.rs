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

use crate::{interaction::Selectable, site::*};
use bevy::prelude::*;
use rmf_site_format::{DoorMarker, Edge, DEFAULT_LEVEL_HEIGHT};

pub const DEFAULT_DOOR_THICKNESS: f32 = 0.1;

#[derive(Debug, Clone, Copy, Component)]
pub struct DoorSegments {
    // TODO(MXG): When it's time to animate the doors we should replace this
    // with an enum for the different possible door types: Single/Double Swing/Sliding
    pub entity: Entity,
}

fn make_door_transforms(
    edge: &Edge<Entity>,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
) -> (Transform, Transform) {
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
    )
}

pub fn add_door_visuals(
    mut commands: Commands,
    new_doors: Query<(Entity, &Edge<Entity>), Added<DoorMarker>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut dependents: Query<&mut AnchorDependents>,
    assets: Res<SiteAssets>,
) {
    for (e, edge) in &new_doors {
        let (pose_tf, shape_tf) = make_door_transforms(edge, &anchors);

        let mut commands = commands.entity(e);
        let child = commands.add_children(|parent| {
            parent
                .spawn_bundle(PbrBundle {
                    mesh: assets.box_mesh.clone(),
                    material: assets.door_material.clone(),
                    transform: shape_tf,
                    ..default()
                })
                .insert(Selectable::new(e))
                .id()
        });

        commands
            .insert_bundle(SpatialBundle {
                transform: pose_tf,
                ..default()
            })
            .insert(DoorSegments { entity: child })
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
    segments: &DoorSegments,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
    transforms: &mut Query<&mut Transform>,
) {
    let (pose_tf, shape_tf) = make_door_transforms(edge, anchors);
    let mut door_transform = transforms.get_mut(entity).unwrap();
    *door_transform = pose_tf;
    let mut shape_transform = transforms.get_mut(segments.entity).unwrap();
    *shape_transform = shape_tf;
}

pub fn update_changed_door(
    doors: Query<(Entity, &Edge<Entity>, &DoorSegments), Changed<Edge<Entity>>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut transforms: Query<&mut Transform>,
) {
    for (entity, edge, segments) in &doors {
        update_door_visuals(entity, edge, segments, &anchors, &mut transforms);
    }
}

pub fn update_door_for_changed_anchor(
    doors: Query<(Entity, &Edge<Entity>, &DoorSegments), With<DoorMarker>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    changed_anchors: Query<&AnchorDependents, (With<Anchor>, Changed<GlobalTransform>)>,
    mut transforms: Query<&mut Transform>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((entity, edge, segments)) = doors.get(*dependent).ok() {
                update_door_visuals(entity, edge, segments, &anchors, &mut transforms);
            }
        }
    }
}
