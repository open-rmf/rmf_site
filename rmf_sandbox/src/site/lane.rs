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
use rmf_site_format::Lane;
use crate::{
    site::*,
    interaction::Selectable,
};

// TODO(MXG): Make these configurable, perhaps even a field in the Lane data
// so users can customize the lane width per lane.
pub const PASSIVE_LANE_HEIGHT: f32 = 0.001;
pub const SELECTED_LANE_HEIGHT: f32 = 0.002;
pub const HOVERED_LANE_HEIGHT: f32 = 0.003;
pub const LANE_WIDTH: f32 = 0.5;

#[derive(Component, Debug, Clone, Copy)]
pub struct LaneSegments {
    pub start: Entity,
    pub mid: Entity,
    pub end: Entity,
}

impl LaneSegments {
    pub fn iter(&self) -> impl Iterator<Item=Entity> {
        [self.start, self.mid, self.end].into_iter()
    }
}

fn should_display_lane(
    lane: &Lane<Entity>,
    parents: &Query<&Parent, With<Anchor>>,
    current_level: &CurrentLevel,
) -> bool {
    for anchor in [lane.anchors.0, lane.anchors.1] {
        if let Ok(level) = parents.get(anchor) {
            if Some(level.get()) == current_level.0 {
                return true;
            }
        }
    }

    return false;
}

pub fn add_lane_visuals(
    mut commands: Commands,
    lanes: Query<(Entity, &Lane<Entity>), Added<Lane<Entity>>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut dependencies: Query<&mut AnchorDependents>,
    parents: Query<&Parent, With<Anchor>>,
    assets: Res<SiteAssets>,
    current_level: Res<CurrentLevel>,
) {
    for (e, new_lane) in &lanes {
        let start_anchor = anchors.get(new_lane.anchors.0).unwrap();
        let end_anchor = anchors.get(new_lane.anchors.1).unwrap();
        let is_visible = should_display_lane(new_lane, &parents, current_level.as_ref());

        let mut commands = commands.entity(e);
        let (start, mid, end) = commands.add_children(|parent| {
            let start = parent
                .spawn_bundle(PbrBundle {
                    mesh: assets.lane_end_mesh.clone(),
                    material: assets.passive_lane_material.clone(),
                    transform: start_anchor.compute_transform(),
                    ..default()
                })
                .insert(Selectable::new(e))
                .id();

            let mid = parent
                .spawn_bundle(PbrBundle {
                    mesh: assets.lane_mid_mesh.clone(),
                    material: assets.passive_lane_material.clone(),
                    transform: line_stroke_transform(start_anchor, end_anchor),
                    ..default()
                })
                .insert(Selectable::new(e))
                .id();

            let end = parent
                .spawn_bundle(PbrBundle {
                    mesh: assets.lane_end_mesh.clone(),
                    material: assets.passive_lane_material.clone(),
                    transform: end_anchor.compute_transform(),
                    ..default()
                })
                .insert(Selectable::new(e))
                .id();

            (start, mid, end)
        });

        commands
            .insert(LaneSegments{start, mid, end})
            .insert_bundle(SpatialBundle{
                transform: Transform::from_translation([0., 0., PASSIVE_LANE_HEIGHT].into()),
                visibility: Visibility{is_visible},
                ..default()
            });
    }
}

fn update_lane_visuals(
    lane: &Lane<Entity>,
    segments: &LaneSegments,
    anchors: &Query<&GlobalTransform, With<Anchor>>,
    transforms: &mut Query<&mut Transform>,
) {
    let start_anchor = anchors.get(lane.anchors.0).unwrap();
    let end_anchor = anchors.get(lane.anchors.1).unwrap();

    if let Some(mut tf) = transforms.get_mut(segments.start).ok() {
        *tf = start_anchor.compute_transform();
    }
    if let Some(mut tf) = transforms.get_mut(segments.mid).ok() {
        *tf = line_stroke_transform(start_anchor, end_anchor);
    }
    if let Some(mut tf) = transforms.get_mut(segments.end).ok() {
        *tf = end_anchor.compute_transform();
    }
}

pub fn update_changed_lane(
    mut lanes: Query<(&Lane<Entity>, &LaneSegments, &mut Visibility), Changed<Lane<Entity>>>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    mut transforms: Query<&mut Transform>,
    parents: Query<&Parent, With<Anchor>>,
    current_level: Res<CurrentLevel>,
) {
    for (lane, segments, mut visibility) in &mut lanes {
        update_lane_visuals(lane, segments, &anchors, &mut transforms);

        let is_visible = should_display_lane(lane, &parents, current_level.as_ref());
        if visibility.is_visible != is_visible {
            visibility.is_visible = is_visible;
        }
    }
}

pub fn update_lane_for_changed_anchor(
    lanes: Query<(&Lane<Entity>, &LaneSegments)>,
    anchors: Query<&GlobalTransform, With<Anchor>>,
    changed_anchors: Query<&AnchorDependents, (With<Anchor>, Changed<GlobalTransform>)>,
    mut transforms: Query<&mut Transform>,
) {
    for changed_anchor in &changed_anchors {
        for dependent in &changed_anchor.dependents {
            if let Some((lane, segments)) = lanes.get(*dependent).ok() {
                update_lane_visuals(lane, segments, &anchors, &mut transforms);
            }
        }
    }
}

pub fn update_lanes_for_changed_level(
    mut lanes: Query<(&Lane<Entity>, &mut Visibility), With<Lane<Entity>>>,
    parents: Query<&Parent, With<Anchor>>,
    current_level: Res<CurrentLevel>,
) {
    if current_level.is_changed() {
        for (lane, mut visibility) in &mut lanes {
            let is_visible = should_display_lane(lane, &parents, current_level.as_ref());
            if visibility.is_visible != is_visible {
                visibility.is_visible = is_visible;
            }
        }
    }
}
