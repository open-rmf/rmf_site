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

use bevy::prelude::*;
use rmf_site_format::Door;
use crate::{
    site::*,
    interaction::Selectable,
};

pub const DEFAULT_DOOR_THICKNESS: f32 = 0.1;

#[derive(Debug, Clone, Copy, Component)]
pub struct DoorSegments {
    // TODO(MXG): When it's time to animate the doors we should replace this
    // with an enum for the different possible door types: Single/Double Swing/Sliding
    pub entity: Entity,
}

fn make_door_transforms(
    door: &Door<Entity>,
    anchors: &Query<&Anchor>,
) -> (Transform, Transform) {
    let start_anchor = anchors.get(door.anchors.0).unwrap();
    let end_anchor = anchors.get(door.anchors.1).unwrap();

    let p_start = start_anchor.vec();
    let p_end = end_anchor.vec();
    let dp = p_start - p_end;
    let length = dp.length();
    let yaw = dp.x.atan2(dp.y);
    let center = (p_start+p_end)/2.0;

    (
        Transform{
            translation: Vec3::new(center.x, center.y, 0.),
            rotation: Quat::from_rotation_z(yaw),
            ..default()
        },
        Transform{
            translation: Vec3::new(0., 0., DEFAULT_LEVEL_HEIGHT/2.0),
            scale: Vec3::new(DEFAULT_DOOR_THICKNESS, length, DEFAULT_LEVEL_HEIGHT),
            ..default()
        }
    )
}

fn add_door_visuals(
    mut commands: Commands,
    doors: Query<(Entity, &Door<Entity>), Added<Door<Entity>>>,
    anchors: Query<&Anchor>,
    assets: Res<SiteAssets>,
) {
    for (e, new_door) in &doors {
        let (pose_tf, shape_tf) = make_door_transforms(new_door, &anchors);

        let mut commands = commands.entity(e);
        let child = commands.add_children(|parent| {
            parent.spawn_bundle(PbrBundle{
                mesh: assets.box_mesh.clone(),
                material: assets.door_material.clone(),
                transform: shape_tf,
                ..default()
            })
            .insert(Selectable::new(e))
            .id()
        });

        commands.insert_bundle(SpatialBundle{
            transform: pose_tf,
            ..default()
        })
        .insert(DoorSegments{entity: child});
    }
}

fn update_door_visuals(
    entity: Entity,
    door: &Door<Entity>,
    segments: &DoorSegments,
    anchors: &Query<&Anchor>,
    transforms: &mut Query<&mut Transform>,
) {
    let (pose_tf, shape_tf) = make_door_transforms(door, anchors);
    let door_transform = transforms.get_mut(entity).unwrap();
    *door_transform = pose_tf;
    let shape_transform = transforms.get_mut(segments.entity).unwrap();
    *shape_transform = shape_tf;
}

fn update_changed_door(
    doors: Query<(Entity, &Door<Entity>, &DoorSegments), Changed<Door<Entity>>>,
    anchors: Query<&Anchor>,
    mut transforms: Query<&mut Transform>,
) {
    for (entity, door, segments) in &doors {
        update_door_visuals(entity, door, segments, &anchors, &mut transforms);
    }
}

fn update_door_for_changed_anchor(
    doors: Query<(Entity, &Door<Entity>, &DoorSegments)>,
    anchors: Query<&Anchor>,
    changed_anchors: Query<&AnchorDependents, Changed<Anchor>>,
    mut transforms: Query<&mut Transform>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((entity, door, segments)) = doors.get(*dependent).ok() {
                update_door_visuals(entity, door, segments, &anchors, &mut transforms);
            }
        }
    }
}
