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
use bevy::{prelude::*, render::primitives::Aabb};
use rmf_site_format::{Edge, LiftCabin};

#[derive(Clone, Copy, Debug, Component)]
pub struct LiftSegments {
    pub cabin: Entity,
}

fn make_lift_transforms(
    reference_anchors: &Edge<Entity>,
    cabin: &LiftCabin<Entity>,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
) -> (Transform, Transform) {
    let start_anchor = anchors.get(reference_anchors.start()).unwrap();
    let end_anchor = anchors.get(reference_anchors.end()).unwrap();
    let (p_start, p_end) = if reference_anchors.left() == reference_anchors.right() {
        (
            start_anchor.translation(),
            start_anchor.translation() + DEFAULT_CABIN_WIDTH * Vec3::Y,
        )
    } else {
        (start_anchor.translation(), end_anchor.translation())
    };

    let dp = p_start - p_end;
    let length = dp.length();
    let yaw = (-dp.x).atan2(dp.y);
    let center = (p_start + p_end) / 2.0;

    let lift_tf = Transform {
        translation: Vec3::new(center.x, center.y, 0.),
        rotation: Quat::from_rotation_z(yaw),
        ..default()
    };

    let cabin_tf = match &cabin {
        LiftCabin::Rect(params) => {
            let Aabb {
                center,
                half_extents,
            } = params.aabb();
            Transform {
                translation: center.into(),
                scale: (2.0 * half_extents).into(),
                ..default()
            }
        }
        // LiftCabin::Model(_) => {
        //     // TODO(MXG): Add proper support for model lifts
        //     Transform::default()
        // }
    };

    (lift_tf, cabin_tf)
}

pub fn add_lift_visuals(
    mut commands: Commands,
    lifts: Query<(Entity, &Edge<Entity>, &LiftCabin<Entity>), Added<LiftCabin<Entity>>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut dependents: Query<&mut AnchorDependents>,
    assets: Res<SiteAssets>,
) {
    for (e, edge, cabin) in &lifts {
        let (pose_tf, shape_tf) = make_lift_transforms(edge, cabin, &anchors);

        let mut commands = commands.entity(e);
        let child = commands.add_children(|parent| {
            parent
                .spawn_bundle(PbrBundle {
                    mesh: assets.box_mesh.clone(),
                    material: assets.door_body_material.clone(),
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
            .insert(LiftSegments { cabin: child })
            .insert(Category::Lift)
            .insert(EdgeLabels::LeftRight);

        for anchor in edge.array() {
            let mut dep = dependents.get_mut(anchor).unwrap();
            dep.dependents.insert(e);
        }
    }
}

fn update_lift_visuals(
    entity: Entity,
    edge: &Edge<Entity>,
    cabin: &LiftCabin<Entity>,
    segments: &LiftSegments,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
    transforms: &mut Query<&mut Transform>,
) {
    let (pose_tf, shape_tf) = make_lift_transforms(edge, cabin, anchors);
    let mut lift_transform = transforms.get_mut(entity).unwrap();
    *lift_transform = pose_tf;
    let mut cabin_transform = transforms.get_mut(segments.cabin).unwrap();
    *cabin_transform = shape_tf;
}

pub fn update_changed_lift(
    lifts: Query<(Entity, &Edge<Entity>, &LiftCabin<Entity>, &LiftSegments), Changed<Edge<Entity>>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut transforms: Query<&mut Transform>,
) {
    for (entity, edge, cabin, segments) in &lifts {
        update_lift_visuals(entity, edge, cabin, segments, &anchors, &mut transforms);
    }
}

pub fn update_lift_for_changed_anchor(
    lifts: Query<(Entity, &Edge<Entity>, &LiftCabin<Entity>, &LiftSegments)>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    changed_anchors: Query<&AnchorDependents, Changed<GlobalTransform>>,
    mut transforms: Query<&mut Transform>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((entity, edge, cabin, segments)) = lifts.get(*dependent).ok() {
                update_lift_visuals(entity, edge, cabin, segments, &anchors, &mut transforms);
            }
        }
    }
}
