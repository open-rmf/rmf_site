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

use crate::site::*;
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
    pub outlines: [Entity; 3],
}

fn should_display_lane(
    edge: &Edge<Entity>,
    associated: &AssociatedGraphs<Entity>,
    parents: &Query<&Parent>,
    levels: &Query<(), With<LevelProperties>>,
    current_level: &Res<CurrentLevel>,
    graphs: &Query<(Entity, &Visibility), With<NavGraphMarker>>,
) -> bool {
    for anchor in edge.array() {
        if let Ok(parent) = parents.get(anchor) {
            if levels.contains(parent.get()) && Some(parent.get()) != ***current_level {
                return false;
            }
        }
    }

    match associated {
        AssociatedGraphs::All =>
            graphs.iter().find(|(_, v)| v.is_visible).is_some(),
        AssociatedGraphs::Only(set) =>
            set.is_empty() || set.iter().find(
                |e| graphs.get(**e).ok().filter(|(_, v)| v.is_visible
            ).is_some()).is_some(),
        AssociatedGraphs::AllExcept(set) =>
            graphs.iter().find(|(e, v)| v.is_visible && !set.contains(e)).is_some(),
    }
}

pub fn add_lane_visuals(
    mut commands: Commands,
    lanes: Query<(Entity, &Edge<Entity>, &AssociatedGraphs<Entity>), Added<LaneMarker>>,
    graph_mats: Query<(Entity, &Handle<StandardMaterial>, &Visibility), With<NavGraphMarker>>,
    graph_vis: Query<(Entity, &Visibility), With<NavGraphMarker>>,
    anchors: AnchorParams,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
    current_level: Res<CurrentLevel>,
) {
    for (e, new_lane, associated_graphs) in &lanes {
        for anchor in &new_lane.array() {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(e);
            }
        }

        let lane_material = match associated_graphs {
            AssociatedGraphs::All =>
                graph_mats
                    .iter()
                    .filter(|(_, _, v)| v.is_visible)
                    .min_by(|(a, _, _), (b, _, _)| a.cmp(b))
                    .map(|(_, m, _)| m)
                    .unwrap_or(&assets.unassigned_lane_material),
            AssociatedGraphs::Only(set) =>
                set
                    .iter()
                    .find(|e| graph_mats.get(**e).ok().filter(|(_, _, v)| v.is_visible).is_some())
                    .map(|e| graph_mats.get(*e).map(|(_, m, _)| m).ok())
                    .flatten()
                    .unwrap_or(&assets.unassigned_lane_material),
            AssociatedGraphs::AllExcept(set) =>
                graph_mats
                    .iter()
                    .filter(|(e, _, v)| v.is_visible && !set.contains(e))
                    .min_by(|(a, _, _), (b, _, _)| a.cmp(b))
                    .map(|(_, m, _)| m)
                    .unwrap_or(&assets.unassigned_lane_material),
        };

        let is_visible = should_display_lane(
            new_lane, associated_graphs, &parents, &levels, &current_level, &graph_vis
        );

        let start_anchor = anchors
            .point_in_parent_frame_of(new_lane.start(), Category::Lane, e)
            .unwrap();
        let end_anchor = anchors
            .point_in_parent_frame_of(new_lane.end(), Category::Lane, e)
            .unwrap();
        let mut commands = commands.entity(e);
        let (start, mid, end, outlines) = commands.add_children(|parent| {
            let mut start = parent.spawn_bundle(PbrBundle {
                mesh: assets.lane_end_mesh.clone(),
                material: lane_material.clone(),
                transform: Transform::from_translation(start_anchor),
                ..default()
            });
            let start_outline = start.add_children(|start| {
                start
                    .spawn_bundle(PbrBundle {
                        mesh: assets.lane_end_outline.clone(),
                        transform: Transform::from_translation(-0.000_5 * Vec3::Z),
                        visibility: Visibility { is_visible: false },
                        ..default()
                    })
                    .id()
            });
            let start = start.id();

            let mut mid = parent.spawn_bundle(PbrBundle {
                mesh: assets.lane_mid_mesh.clone(),
                material: lane_material.clone(),
                transform: line_stroke_transform(&start_anchor, &end_anchor, LANE_WIDTH),
                ..default()
            });
            let mid_outline = mid.add_children(|mid| {
                mid.spawn_bundle(PbrBundle {
                    mesh: assets.lane_mid_outline.clone(),
                    transform: Transform::from_translation(-0.000_5 * Vec3::Z),
                    visibility: Visibility { is_visible: false },
                    ..default()
                })
                .id()
            });
            let mid = mid.id();

            let mut end = parent.spawn_bundle(PbrBundle {
                mesh: assets.lane_end_mesh.clone(),
                material: lane_material.clone(),
                transform: Transform::from_translation(end_anchor),
                ..default()
            });
            let end_outline = end.add_children(|end| {
                end.spawn_bundle(PbrBundle {
                    mesh: assets.lane_end_outline.clone(),
                    transform: Transform::from_translation(-0.000_5 * Vec3::Z),
                    visibility: Visibility { is_visible: false },
                    ..default()
                })
                .id()
            });
            let end = end.id();

            (start, mid, end, [start_outline, mid_outline, end_outline])
        });

        commands
            .insert(LaneSegments {
                start,
                mid,
                end,
                outlines,
            })
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
    let start_anchor = anchors
        .point_in_parent_frame_of(edge.left(), Category::Lane, entity)
        .unwrap();
    let end_anchor = anchors
        .point_in_parent_frame_of(edge.right(), Category::Lane, entity)
        .unwrap();

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
    mut lanes: Query<
        (Entity, &Edge<Entity>, &AssociatedGraphs<Entity>, &LaneSegments, &mut Visibility),
        (Changed<Edge<Entity>>, Without<NavGraphMarker>),
    >,
    anchors: AnchorParams,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    graph_vis: Query<(Entity, &Visibility), With<NavGraphMarker>>,
    mut transforms: Query<&mut Transform>,
    current_level: Res<CurrentLevel>,
) {
    for (e, edge, associated, segments, mut visibility) in &mut lanes {
        update_lane_visuals(e, edge, segments, &anchors, &mut transforms);

        let is_visible = should_display_lane(
            edge, associated, &parents, &levels, &current_level, &graph_vis
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
    mut lanes: Query<(&Edge<Entity>, &AssociatedGraphs<Entity>, &mut Visibility), (With<LaneMarker>, Without<NavGraphMarker>)>,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    current_level: Res<CurrentLevel>,
    graph_vis: Query<(Entity, &Visibility), With<NavGraphMarker>>,
    lanes_with_changed_association: Query<Entity, (With<LaneMarker>, Changed<AssociatedGraphs<Entity>>)>,
    graph_changed_visibility: Query<(), (With<NavGraphMarker>, Changed<Visibility>)>,
) {
    let update_all = current_level.is_changed() || !graph_changed_visibility.is_empty();
    if update_all {
        for (edge, associated, mut visibility) in &mut lanes {
            let is_visible = should_display_lane(
                edge, associated, &parents, &levels, &current_level, &graph_vis
            );
            if visibility.is_visible != is_visible {
                visibility.is_visible = is_visible;
            }
        }
    } else {
        for e in &lanes_with_changed_association {
            if let Ok((edge, associated, mut visibility)) = lanes.get_mut(e) {
                let is_visible = should_display_lane(
                    edge, associated, &parents, &levels, &current_level, &graph_vis
                );
                if visibility.is_visible != is_visible {
                    visibility.is_visible = is_visible;
                }
            }
        }
    }
}
