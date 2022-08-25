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
use rmf_site_format::{Lift, LiftCabin, DEFAULT_CABIN_WALL_THICKNESS, DEFAULT_CABIN_GAP};
use crate::{
    site::*,
    interaction::Selectable,
};

#[derive(Clone, Copy, Debug, Component)]
pub struct LiftSegments {
    pub cabin: Entity,
}

fn make_lift_transforms(
    lift: &Lift<Entity>,
    anchors: &Query<&Anchor>,
) -> (Transform, Transform) {
    let start_anchor = anchors.get(lift.reference_anchors.0).unwrap();
    let end_anchor = anchors.get(lift.reference_anchors.1).unwrap();

    let p_start = start_anchor.vec();
    let p_end = end_anchor.vec();
    let dp = p_start - p_end;
    let length = dp.length();
    let yaw = dp.x.atan2(dp.y);
    let center = (p_start+p_end)/2.0;

    let lift_tf = Transform{
        translation: Vec3::new(center.x, center.y, 0.),
        rotation: Quat::from_rotation_z(yaw),
        ..default()
    };

    let cabin_tf = match &lift.cabin {
        LiftCabin::Params{width, depth, door, wall_thickness, gap, shift} => {
            let thick = wall_thickness.unwrap_or(DEFAULT_CABIN_WALL_THICKNESS);
            let gap = gap.unwrap_or(DEFAULT_CABIN_GAP);
            let x = -depth/2.0 - thick - gap;
            let y = shift.unwrap_or(0.);

            Transform{
                translation: Vec3::new(x, y, DEFAULT_LEVEL_HEIGHT/2.0),
                scale: Vec3::new(depth, width, DEFAULT_LEVEL_HEIGHT),
                ..default()
            }
        },
        LiftCabin::Model(_) => {
            Transform::default()
        }
    };

    (lift_tf, cabin_tf)
}

pub fn add_lift_visuals(
    mut commands: Commands,
    lifts: Query<(Entity, &Lift<Entity>), Added<Lift<Entity>>>,
    anchors: Query<&Anchor>,
    assets: Res<SiteAssets>,
) {
    for (e, lift) in &lifts {
        let (pose_tf, shape_tf) = make_lift_transforms(lift, &anchors);

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
        .insert(LiftSegments{cabin: child});
    }
}

fn update_lift_visuals(
    entity: Entity,
    lift: &Lift<Entity>,
    segments: &LiftSegments,
    anchors: &Query<&Anchor>,
    transforms: &mut Query<&mut Transform>,
) {
    let (pose_tf, shape_tf) = make_lift_transforms(lift, anchors);
    let lift_transform = transforms.get_mut(entity).unwrap();
    *lift_transform = pose_tf;
    let cabin_transform = transforms.get_mut(segments.cabin).unwrap();
    *cabin_transform = shape_tf;
}

pub fn update_changed_lift(
    lifts: Query<(Entity, &Lift<Entity>, &LiftSegments), Changed<Lift<Entity>>>,
    anchors: Query<&Anchor>,
    mut transforms: Query<&mut Transform>,
) {
    for (entity, lift, segments) in &lifts {
        update_lift_visuals(entity, lift, segments, &anchors, &mut transforms);
    }
}

pub fn update_lift_for_changed_anchor(
    lifts: Query<(Entity, &Lift<Entity>, &LiftSegments)>,
    anchors: Query<&Anchor>,
    changed_anchors: Query<&AnchorDependents, Changed<Anchor>>,
    mut transforms: Query<&mut Transform>
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((entity, lift, segments)) = lifts.get(*dependent).ok() {
                update_lift_visuals(entity, lift, segments, &anchors, &mut transforms);
            }
        }
    }
}
