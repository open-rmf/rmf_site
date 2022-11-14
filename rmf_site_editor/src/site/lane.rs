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
    interaction::{Cursor, Selectable},
    site::*,
};
use bevy::prelude::*;
use rmf_site_format::{Edge, LaneMarker};

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
    pub fn iter(&self) -> impl Iterator<Item = Entity> {
        [self.start, self.mid, self.end].into_iter()
    }
}

fn should_display_lane(
    edge: &Edge<Entity>,
    parents: &Query<&Parent>,
    levels: &Query<(), With<LevelProperties>>,
    current_level: &Res<CurrentLevel>,
) -> bool {
    for anchor in edge.array() {
        if let Ok(parent) = parents.get(anchor) {
            if levels.contains(parent.get()) && Some(parent.get()) != ***current_level {
                return false;
            }
        }
    }

    return true;
}

pub fn add_lane_visuals(
    mut commands: Commands,
    lanes: Query<(Entity, &Edge<Entity>), Added<LaneMarker>>,
    anchors: AnchorParams,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
    current_level: Res<CurrentLevel>,
) {
    for (e, new_lane) in &lanes {
        for mut anchor in &new_lane.array() {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(e);
            }
        }

        let is_visible = should_display_lane(
            new_lane,
            &parents,
            &levels,
            &current_level,
        );

        let start_anchor = anchors.point_in_parent_frame_of(new_lane.start(), Category::Lane, e).unwrap();
        let end_anchor = anchors.point_in_parent_frame_of(new_lane.end(), Category::Lane, e).unwrap();
        let mut commands = commands.entity(e);
        let (start, mid, end) = commands.add_children(|parent| {
            let start = parent
                .spawn_bundle(PbrBundle {
                    mesh: assets.lane_end_mesh.clone(),
                    material: assets.passive_lane_material.clone(),
                    transform: Transform::from_translation(start_anchor),
                    ..default()
                })
                .id();

            let mid = parent
                .spawn_bundle(PbrBundle {
                    mesh: assets.lane_mid_mesh.clone(),
                    material: assets.passive_lane_material.clone(),
                    transform: line_stroke_transform(&start_anchor, &end_anchor, LANE_WIDTH),
                    ..default()
                })
                .insert(Selectable::new(e))
                .id();

            let end = parent
                .spawn_bundle(PbrBundle {
                    mesh: assets.lane_end_mesh.clone(),
                    material: assets.passive_lane_material.clone(),
                    transform: Transform::from_translation(end_anchor),
                    ..default()
                })
                .id();

            (start, mid, end)
        });

        commands
            .insert(LaneSegments { start, mid, end })
            .insert_bundle(SpatialBundle {
                transform: Transform::from_translation([0., 0., PASSIVE_LANE_HEIGHT].into()),
                visibility: Visibility { is_visible },
                ..default()
            })
            .insert(Category::Lane)
            .insert(EdgeLabels::StartEnd);
    }
}

fn update_lane_visuals(
    entity: Entity,
    edge: &Edge<Entity>,
    segments: &LaneSegments,
    anchors: &AnchorParams,
    transforms: &mut Query<&mut Transform>,
) {
    let start_anchor = anchors.point_in_parent_frame_of(edge.left(), Category::Lane, entity).unwrap();
    let end_anchor = anchors.point_in_parent_frame_of(edge.right(), Category::Lane, entity).unwrap();

    if let Some(mut tf) = transforms.get_mut(segments.start).ok() {
        *tf = Transform::from_translation(start_anchor);
    }
    if let Some(mut tf) = transforms.get_mut(segments.mid).ok() {
        *tf = line_stroke_transform(&start_anchor, &end_anchor, LANE_WIDTH);
    }
    if let Some(mut tf) = transforms.get_mut(segments.end).ok() {
        *tf = Transform::from_translation(end_anchor);
    }
}

pub fn update_changed_lane(
    mut lanes: Query<(Entity, &Edge<Entity>, &LaneSegments, &mut Visibility), Changed<Edge<Entity>>>,
    anchors: AnchorParams,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    mut transforms: Query<&mut Transform>,
    current_level: Res<CurrentLevel>,
) {
    for (e, edge, segments, mut visibility) in &mut lanes {
        update_lane_visuals(e, edge, segments, &anchors, &mut transforms);

        let is_visible = should_display_lane(
            edge,
            &parents,
            &levels,
            &current_level,
        );
        if visibility.is_visible != is_visible {
            visibility.is_visible = is_visible;
        }
    }
}

pub fn update_lane_for_moved_anchor(
    lanes: Query<(Entity, &Edge<Entity>, &LaneSegments)>,
    anchors: AnchorParams,
    changed_anchors: Query<&Dependents, (With<Anchor>, Changed<GlobalTransform>)>,
    mut transforms: Query<&mut Transform>,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Some((e, edge, segments)) = lanes.get(*dependent).ok() {
                update_lane_visuals(e, edge, segments, &anchors, &mut transforms);
            }
        }
    }
}

// TODO(MXG): Generalize this to all edges
pub fn update_visibility_for_lanes(
    mut lanes: Query<(&Edge<Entity>, &mut Visibility), With<LaneMarker>>,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    current_level: Res<CurrentLevel>,
) {
    if current_level.is_changed() {
        for (edge, mut visibility) in &mut lanes {
            let is_visible = should_display_lane(
                edge,
                &parents,
                &levels,
                &current_level,
            );
            if visibility.is_visible != is_visible {
                visibility.is_visible = is_visible;
            }
        }
    }
}
