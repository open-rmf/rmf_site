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
use crate::{CurrentWorkspace, Issue, ValidateWorkspace};
use bevy::{prelude::*, utils::Uuid};
use rmf_site_format::{Edge, LaneMarker};
use std::collections::{BTreeSet, HashMap};

pub const SELECTED_LANE_OFFSET: f32 = 0.001;
pub const HOVERED_LANE_OFFSET: f32 = 0.002;
pub const LANE_LAYER_START: f32 = FLOOR_LAYER_START + 0.001;
pub const LANE_LAYER_LIMIT: f32 = LANE_LAYER_START + SELECTED_LANE_OFFSET;

// TODO(MXG): Make this configurable, perhaps even a field in the Lane data
// so users can customize the lane width per lane.
pub const LANE_WIDTH: f32 = 0.5;

#[derive(Component, Debug, Clone, Copy)]
pub struct LaneSegments {
    pub layer: Entity,
    pub start: Entity,
    pub mid: Entity,
    pub end: Entity,
    pub outlines: [Entity; 3],
}

impl LaneSegments {
    pub fn iter(&self) -> [Entity; 3] {
        [self.start, self.mid, self.end]
    }
}

// TODO(MXG): Refactor these function arguments into a SystemParam
fn should_display_lane(
    edge: &Edge<Entity>,
    associated: &AssociatedGraphs<Entity>,
    parents: &Query<&Parent>,
    levels: &Query<(), With<LevelProperties>>,
    current_level: &Res<CurrentLevel>,
    graphs: &GraphSelect,
) -> bool {
    for anchor in edge.array() {
        if let Ok(parent) = parents.get(anchor) {
            if levels.contains(parent.get()) && Some(parent.get()) != ***current_level {
                return false;
            }
        }
    }

    graphs.should_display(associated)
}

pub fn assign_orphan_nav_elements_to_site(
    mut commands: Commands,
    elements: Query<
        Entity,
        (
            Without<Parent>,
            Or<(With<LaneMarker>, With<LocationTags>, With<NavGraphMarker>)>,
        ),
    >,
    current_workspace: Res<CurrentWorkspace>,
    open_sites: Query<Entity, With<SiteProperties<Entity>>>,
) {
    if let Some(current_site) = current_workspace.to_site(&open_sites) {
        for e in &elements {
            commands.entity(current_site).add_child(e);
        }
    }
}

pub fn add_lane_visuals(
    mut commands: Commands,
    lanes: Query<(Entity, &Edge<Entity>, &AssociatedGraphs<Entity>), Added<LaneMarker>>,
    graphs: GraphSelect,
    anchors: AnchorParams,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
    current_level: Res<CurrentLevel>,
) {
    for (e, edge, associated_graphs) in &lanes {
        for anchor in &edge.array() {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(e);
            }
        }

        let (lane_material, height) = graphs.display_style(associated_graphs);
        let is_visible = should_display_lane(
            edge,
            associated_graphs,
            &parents,
            &levels,
            &current_level,
            &graphs,
        );

        let start_anchor = anchors
            .point_in_parent_frame_of(edge.start(), Category::Lane, e)
            .unwrap();
        let end_anchor = anchors
            .point_in_parent_frame_of(edge.end(), Category::Lane, e)
            .unwrap();
        let mut commands = commands.entity(e);
        let (layer, start, mid, end, outlines) = commands.add_children(|parent| {
            // Create a "layer" entity that manages the height of the lane,
            // determined by the DisplayHeight of the graph.
            let mut layer_cmd = parent.spawn(SpatialBundle {
                transform: Transform::from_xyz(0.0, 0.0, height),
                ..default()
            });

            let (start, mid, end, outlines) = layer_cmd.add_children(|parent| {
                let mut start = parent.spawn(PbrBundle {
                    mesh: assets.lane_end_mesh.clone(),
                    material: lane_material.clone(),
                    transform: Transform::from_translation(start_anchor),
                    ..default()
                });
                let start_outline = start.add_children(|start| {
                    start
                        .spawn(PbrBundle {
                            mesh: assets.lane_end_outline.clone(),
                            transform: Transform::from_translation(-0.000_5 * Vec3::Z),
                            visibility: Visibility { is_visible: false },
                            ..default()
                        })
                        .id()
                });
                let start = start.id();

                let mut mid = parent.spawn(PbrBundle {
                    mesh: assets.lane_mid_mesh.clone(),
                    material: lane_material.clone(),
                    transform: line_stroke_transform(&start_anchor, &end_anchor, LANE_WIDTH),
                    ..default()
                });
                let mid_outline = mid.add_children(|mid| {
                    mid.spawn(PbrBundle {
                        mesh: assets.lane_mid_outline.clone(),
                        transform: Transform::from_translation(-0.000_5 * Vec3::Z),
                        visibility: Visibility { is_visible: false },
                        ..default()
                    })
                    .id()
                });
                let mid = mid.id();

                let mut end = parent.spawn(PbrBundle {
                    mesh: assets.lane_end_mesh.clone(),
                    material: lane_material.clone(),
                    transform: Transform::from_translation(end_anchor),
                    ..default()
                });
                let end_outline = end.add_children(|end| {
                    end.spawn(PbrBundle {
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

            (layer_cmd.id(), start, mid, end, outlines)
        });

        commands
            .insert(LaneSegments {
                layer,
                start,
                mid,
                end,
                outlines,
            })
            .insert(SpatialBundle {
                transform: Transform::from_translation([0., 0., LANE_LAYER_START].into()),
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
        (
            Entity,
            &Edge<Entity>,
            &AssociatedGraphs<Entity>,
            &LaneSegments,
            &mut Visibility,
        ),
        (Changed<Edge<Entity>>, Without<NavGraphMarker>),
    >,
    anchors: AnchorParams,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    graphs: GraphSelect,
    mut transforms: Query<&mut Transform>,
    current_level: Res<CurrentLevel>,
) {
    for (e, edge, associated, segments, mut visibility) in &mut lanes {
        update_lane_visuals(e, edge, segments, &anchors, &mut transforms);

        let is_visible =
            should_display_lane(edge, associated, &parents, &levels, &current_level, &graphs);
        if visibility.is_visible != is_visible {
            visibility.is_visible = is_visible;
        }
    }
}

pub fn update_lane_for_moved_anchor(
    lanes: Query<(Entity, &Edge<Entity>, &LaneSegments)>,
    anchors: AnchorParams,
    changed_anchors: Query<
        &Dependents,
        (
            With<Anchor>,
            Or<(Changed<Anchor>, Changed<GlobalTransform>)>,
        ),
    >,
    mut transforms: Query<&mut Transform>,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Ok((e, edge, segments)) = lanes.get(*dependent) {
                update_lane_visuals(e, edge, segments, &anchors, &mut transforms);
            }
        }
    }
}

pub fn remove_association_for_deleted_graphs(
    mut associaged_graphs: Query<&mut AssociatedGraphs<Entity>>,
    removed: RemovedComponents<NavGraphMarker>,
) {
    for e in removed.iter() {
        for mut associated in &mut associaged_graphs {
            match associated.as_mut() {
                AssociatedGraphs::All => {}
                AssociatedGraphs::Only(set) => {
                    set.remove(&e);
                }
                AssociatedGraphs::AllExcept(set) => {
                    set.remove(&e);
                }
            }
        }
    }
}

// TODO(MXG): Generalize this to all edges
pub fn update_visibility_for_lanes(
    mut lanes: Query<
        (
            &Edge<Entity>,
            &AssociatedGraphs<Entity>,
            &LaneSegments,
            &mut Visibility,
        ),
        (With<LaneMarker>, Without<NavGraphMarker>),
    >,
    parents: Query<&Parent>,
    levels: Query<(), With<LevelProperties>>,
    current_level: Res<CurrentLevel>,
    graphs: GraphSelect,
    lanes_with_changed_association: Query<
        (Entity, &AssociatedGraphs<Entity>, &LaneSegments),
        (With<LaneMarker>, Changed<AssociatedGraphs<Entity>>),
    >,
    mut materials: Query<&mut Handle<StandardMaterial>, Without<NavGraphMarker>>,
    mut transforms: Query<&mut Transform>,
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
        for (edge, associated, _, mut visibility) in &mut lanes {
            let is_visible =
                should_display_lane(edge, associated, &parents, &levels, &current_level, &graphs);
            if visibility.is_visible != is_visible {
                visibility.is_visible = is_visible;
            }
        }
    } else {
        for (e, _, _) in &lanes_with_changed_association {
            if let Ok((edge, associated, _, mut visibility)) = lanes.get_mut(e) {
                let is_visible = should_display_lane(
                    edge,
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
        for (_, associated_graphs, segments, _) in &lanes {
            let (mat, height) = graphs.display_style(associated_graphs);
            for e in segments.iter() {
                if let Ok(mut m) = materials.get_mut(e) {
                    *m = mat.clone();
                }
            }

            if let Ok(mut tf) = transforms.get_mut(segments.layer) {
                tf.translation.z = height;
            }
        }
    } else {
        for (_, associated_graphs, segments) in &lanes_with_changed_association {
            let (mat, height) = graphs.display_style(associated_graphs);
            for e in segments.iter() {
                if let Ok(mut m) = materials.get_mut(e) {
                    *m = mat.clone();
                }
            }

            if let Ok(mut tf) = transforms.get_mut(segments.layer) {
                tf.translation.z = height;
            }
        }
    }
}

#[derive(Debug, Clone, Copy)]
pub struct ConsiderAssociatedGraph {
    pub graph: Option<Entity>,
    pub for_element: Entity,
}

impl ConsiderAssociatedGraph {
    pub fn new(graph: Option<Entity>, for_element: Entity) -> Self {
        Self { graph, for_element }
    }
}

pub fn handle_consider_associated_graph(
    mut recalls: Query<&mut RecallAssociatedGraphs<Entity>>,
    mut considerations: EventReader<ConsiderAssociatedGraph>,
) {
    for consider in considerations.iter() {
        if let Ok(mut recall) = recalls.get_mut(consider.for_element) {
            recall.consider = consider.graph;
        }
    }
}

/// Unique UUID to identify issue of duplicated door names
pub const DUPLICATED_DOCK_NAME_ISSUE_UUID: Uuid =
    Uuid::from_u128(0xca210e1025014ac2a4072bc956d76151u128);

// When triggered by a validation request event, check if there are duplicated dock names and
// generate an issue if that is the case
pub fn check_for_duplicated_dock_names(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    parents: Query<&Parent>,
    lane_properties: Query<(Entity, &Motion, Option<&ReverseLane>), With<LaneMarker>>,
) {
    const hint: &str = "RMF uses the dock name parameter to trigger special behavior from \
                        the robots. Duplicated dock names would make such behavior ambiguous as \
                        it would be triggered in different parts of the map, rename the docks to \
                        be unique";
    for root in validate_events.iter() {
        let mut names: HashMap<String, BTreeSet<Entity>> = HashMap::new();
        for (e, motion, reverse) in &lane_properties {
            if AncestorIter::new(&parents, e).any(|p| p == **root) {
                if let Some(dock) = &motion.dock {
                    let entities_with_name = names.entry(dock.name.clone()).or_default();
                    entities_with_name.insert(e);
                }
            }
            if let Some(reverse) = reverse {
                if let ReverseLane::Different(m) = reverse {
                    if let Some(dock) = &m.dock {
                        let entities_with_name = names.entry(dock.name.clone()).or_default();
                        let inserted = entities_with_name.insert(e);
                        if !inserted {
                            let issue = Issue {
                                key: IssueKey {
                                    entities: [e].into(),
                                    kind: DUPLICATED_DOCK_NAME_ISSUE_UUID,
                                },
                                brief: format!(
                                    "Same dock name found for forward and reverse motion {}",
                                    dock.name
                                ),
                                hint: hint.to_string(),
                            };
                            let id = commands.spawn(issue).id();
                            commands.entity(**root).add_child(id);
                        }
                    }
                }
            }
        }
        for (name, entities) in names.drain() {
            if entities.len() > 1 {
                let issue = Issue {
                    key: IssueKey {
                        entities: entities,
                        kind: DUPLICATED_DOCK_NAME_ISSUE_UUID,
                    },
                    brief: format!("Multiple docks found with the same name {}", name),
                    hint: hint.to_string(),
                };
                let id = commands.spawn(issue).id();
                commands.entity(**root).add_child(id);
            }
        }
    }
}
