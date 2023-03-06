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

use crate::{animate::Spinning, interaction::VisualCue, site::*};
use bevy::prelude::*;

// TODO(MXG): Consider using recency rankings for Locations so they don't
// experience z-fighting.
const LOCATION_LAYER_HEIGHT: f32 = LANE_LAYER_LIMIT + SELECTED_LANE_OFFSET / 2.0;

// TODO(MXG): Refactor this implementation with should_display_lane using traits and generics
fn should_display_point(
    point: &Point<Entity>,
    associated: &AssociatedGraphs<Entity>,
    parents: &Query<&Parent>,
    levels: &Query<(), With<LevelProperties>>,
    current_level: &Res<CurrentLevel>,
    graphs: &GraphSelect,
) -> bool {
    if let Ok(parent) = parents.get(point.0) {
        if levels.contains(parent.get()) && Some(parent.get()) != ***current_level {
            return false;
        }
    }

    graphs.should_display(associated)
}

pub fn add_location_visuals(
    mut commands: Commands,
    locations: Query<(Entity, &Point<Entity>, &AssociatedGraphs<Entity>), Added<LocationTags>>,
    graphs: GraphSelect,
    anchors: AnchorParams,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
    current_level: Res<CurrentLevel>,
) {
    for (e, point, associated_graphs) in &locations {
        if let Ok(mut deps) = dependents.get_mut(point.0) {
            deps.insert(e);
        }

        let material = graphs.display_style(associated_graphs).0;
        let is_visible = should_display_point(
            point,
            associated_graphs,
            &parents,
            &levels,
            &current_level,
            &graphs,
        );

        let position = anchors
            .point_in_parent_frame_of(point.0, Category::Location, e)
            .unwrap()
            + LOCATION_LAYER_HEIGHT * Vec3::Z;
        // TODO(MXG): Put icons on the different visual squares based on the location tags
        commands
            .entity(e)
            .insert(PbrBundle {
                mesh: assets.location_mesh.clone(),
                transform: Transform::from_translation(position),
                material,
                visibility: Visibility { is_visible },
                ..default()
            })
            .insert(Spinning::new(-10.0))
            .insert(Category::Location)
            .insert(VisualCue::outline());
    }
}

pub fn update_changed_location(
    mut locations: Query<
        (
            Entity,
            &Point<Entity>,
            &AssociatedGraphs<Entity>,
            &mut Visibility,
            &mut Transform,
        ),
        (Changed<Point<Entity>>, Without<NavGraphMarker>),
    >,
    anchors: AnchorParams,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    graphs: GraphSelect,
    current_level: Res<CurrentLevel>,
) {
    for (e, point, associated, mut visibility, mut tf) in &mut locations {
        let position = anchors
            .point_in_parent_frame_of(point.0, Category::Location, e)
            .unwrap();
        tf.translation = position;
        tf.translation.z = LOCATION_LAYER_HEIGHT;

        let is_visible = should_display_point(
            point,
            associated,
            &parents,
            &levels,
            &current_level,
            &graphs,
        );
        if visibility.is_visible != is_visible {
            visibility.is_visible = is_visible;
        }
    }
}

pub fn update_location_for_moved_anchors(
    mut locations: Query<(Entity, &Point<Entity>, &mut Transform), With<LocationTags>>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Ok((e, point, mut tf)) = locations.get_mut(*dependent) {
                let position = anchors
                    .point_in_parent_frame_of(point.0, Category::Location, e)
                    .unwrap();
                tf.translation = position;
                tf.translation.z = LOCATION_LAYER_HEIGHT;
            }
        }
    }
}

pub fn update_visibility_for_locations(
    mut locations: Query<
        (
            &Point<Entity>,
            &AssociatedGraphs<Entity>,
            &mut Visibility,
            &mut Handle<StandardMaterial>,
            // &mut
        ),
        (With<LocationTags>, Without<NavGraphMarker>),
    >,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    current_level: Res<CurrentLevel>,
    graphs: GraphSelect,
    locations_with_changed_association: Query<
        Entity,
        (With<LocationTags>, Changed<AssociatedGraphs<Entity>>),
    >,
    graph_changed_visibility: Query<
        (),
        (
            With<NavGraphMarker>,
            Or<(Changed<Visibility>, Changed<RecencyRank<NavGraphMarker>>)>,
        ),
    >,
    removed: RemovedComponents<NavGraphMarker>,
) {
    let graph_change = !graph_changed_visibility.is_empty() || removed.iter().next().is_some();
    let update_all = current_level.is_changed() || graph_change;
    if update_all {
        for (point, associated, mut visibility, _) in &mut locations {
            let is_visible = should_display_point(
                point,
                associated,
                &parents,
                &levels,
                &current_level,
                &graphs,
            );
            if visibility.is_visible != is_visible {
                visibility.is_visible = is_visible;
            }
        }
    } else {
        for e in &locations_with_changed_association {
            if let Ok((point, associated, mut visibility, _)) = locations.get_mut(e) {
                let is_visible = should_display_point(
                    point,
                    associated,
                    &parents,
                    &levels,
                    &current_level,
                    &graphs,
                );
                if visibility.is_visible != is_visible {
                    visibility.is_visible = is_visible;
                }
            }
        }
    }

    if graph_change {
        for (_, associated_graphs, _, mut m) in &mut locations {
            *m = graphs.display_style(associated_graphs).0;
        }
    } else {
        for e in &locations_with_changed_association {
            if let Ok((_, associated_graphs, _, mut m)) = locations.get_mut(e) {
                *m = graphs.display_style(associated_graphs).0;
            }
        }
    }
}

#[derive(Debug, Clone)]
pub struct ConsiderLocationTag {
    pub tag: Option<LocationTag>,
    pub for_element: Entity,
}

impl ConsiderLocationTag {
    pub fn new(tag: Option<LocationTag>, for_element: Entity) -> Self {
        Self { tag, for_element }
    }
}

// TODO(MXG): Consider refactoring into a generic plugin, alongside ConsiderAssociatedGraph
pub fn handle_consider_location_tag(
    mut recalls: Query<&mut RecallLocationTags>,
    mut considerations: EventReader<ConsiderLocationTag>,
) {
    for consider in considerations.iter() {
        if let Ok(mut recall) = recalls.get_mut(consider.for_element) {
            recall.consider_tag = consider.tag.clone();
            let r = recall.as_mut();
            if let Some(LocationTag::SpawnRobot(model)) | Some(LocationTag::Workcell(model)) =
                &r.consider_tag
            {
                r.consider_tag_asset_source_recall.remember(&model.source);
            }
        }
    }
}
