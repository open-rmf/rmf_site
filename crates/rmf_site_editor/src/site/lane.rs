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

use crate::{layers::ZLayer, site::*};
use crate::{CurrentWorkspace, Issue, ValidateWorkspace};
use bevy::ecs::{hierarchy::ChildOf, relationship::AncestorIter};
use bevy::pbr::ExtendedMaterial;
use bevy::prelude::*;
use rmf_site_format::{Edge, LaneMarker};
use std::collections::{BTreeSet, HashMap};
use uuid::Uuid;

use bevy::picking::Pickable;

// TODO(MXG): Make this configurable, perhaps even a field in the Lane data
// so users can customize the lane width per lane.
pub const LANE_WIDTH: f32 = 0.5;
pub const DEFAULT_LANE_ARROW_SPEED: f32 = 0.5;

#[derive(Component, Debug, Clone, Copy)]
pub struct LaneSegments {
    /// Offset the lane height based on nav graph rank
    pub rank_layer: Entity,
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
    child_of: &Query<&ChildOf>,
    levels: &Query<(), With<LevelElevation>>,
    current_level: &Res<CurrentLevel>,
    graphs: &GraphSelect,
) -> bool {
    for anchor in edge.array() {
        if let Ok(child_of) = child_of.get(anchor) {
            if levels.contains(child_of.parent()) && Some(child_of.parent()) != ***current_level {
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
            Without<ChildOf>,
            Or<(With<LaneMarker>, With<LocationTags>, With<NavGraphMarker>)>,
        ),
    >,
    current_workspace: Res<CurrentWorkspace>,
    open_sites: Query<Entity, With<NameOfSite>>,
) {
    if let Some(current_site) = current_workspace.to_site(&open_sites) {
        for e in &elements {
            commands.entity(current_site).add_child(e);
        }
    }
}

pub fn add_lane_visuals(
    mut commands: Commands,
    lanes: Query<
        (
            Entity,
            &Motion,
            &ReverseLane,
            Option<&RecallMotion>,
            &Edge<Entity>,
            &AssociatedGraphs<Entity>,
        ),
        Added<LaneMarker>,
    >,
    graphs: GraphSelect,
    anchors: AnchorParams,
    child_of: Query<&ChildOf>,
    levels: Query<(), With<LevelElevation>>,
    mut dependents: Query<&mut Dependents, With<Anchor>>,
    assets: Res<SiteAssets>,
    current_level: Res<CurrentLevel>,
    mut extended_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
) {
    for (e, motion, reverse, recall, edge, associated_graphs) in &lanes {
        for anchor in &edge.array() {
            if let Ok(mut deps) = dependents.get_mut(*anchor) {
                deps.insert(e);
            }
        }

        let (lane_material, lane_color, rank) = graphs.display_style(associated_graphs);
        let visibility = if should_display_lane(
            edge,
            associated_graphs,
            &child_of,
            &levels,
            &current_level,
            &graphs,
        ) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };

        let start_anchor = anchors
            .point_in_parent_frame_of(edge.start(), Category::Lane, e)
            .unwrap();
        let end_anchor = anchors
            .point_in_parent_frame_of(edge.end(), Category::Lane, e)
            .unwrap();

        // Create a "layer" entity that manages the height offset of the lane,
        // determined by its nav graph rank.
        let rank_layer = commands
            .spawn((
                Transform::from_xyz(0.0, 0.0, height_for_rank(rank)),
                Visibility::default(),
            ))
            .insert(ChildOf(e))
            .id();

        let mut spawn_lane_mesh_and_outline = |lane_tf, lane_mesh, outline_mesh| {
            let mesh = commands
                .spawn((
                    Mesh3d(lane_mesh),
                    MeshMaterial3d(lane_material.clone()),
                    lane_tf,
                    Visibility::default(),
                    ChildOf(rank_layer),
                ))
                .id();

            let outline = commands
                .spawn((
                    Mesh3d(outline_mesh),
                    MeshMaterial3d::<StandardMaterial>::default(),
                    Transform::from_translation(-0.000_5 * Vec3::Z),
                    Visibility::Hidden,
                    ChildOf(mesh),
                    Pickable::IGNORE,
                ))
                .id();

            (mesh, outline)
        };

        let (start, start_outline) = spawn_lane_mesh_and_outline(
            Transform::from_translation(start_anchor),
            assets.lane_end_mesh.clone(),
            assets.lane_end_outline.clone(),
        );

        let (end, end_outline) = spawn_lane_mesh_and_outline(
            Transform::from_translation(end_anchor),
            assets.lane_end_mesh.clone(),
            assets.lane_end_outline.clone(),
        );

        let is_bidirectional = *reverse != ReverseLane::Disable;
        let forward_speed_limit = motion.speed_limit.unwrap_or(DEFAULT_LANE_ARROW_SPEED);
        let backward_speed_limit = match reverse {
            ReverseLane::Same => forward_speed_limit,
            _ => recall
                .map(|rec: &RecallMotion| rec.speed_limit.unwrap_or(DEFAULT_LANE_ARROW_SPEED))
                .unwrap_or(DEFAULT_LANE_ARROW_SPEED),
        };

        let lane_color = lane_color.to_linear();
        let mid = commands
            .spawn((
                Mesh3d(assets.lane_mid_mesh.clone()),
                MeshMaterial3d(extended_materials.add(ExtendedMaterial {
                    base: StandardMaterial {
                        depth_bias: 3.0,
                        ..default()
                    },
                    extension: assets::LaneArrowMaterial {
                        single_arrow_color: forward_arrow_color(lane_color),
                        double_arrow_color: backward_arrow_color(lane_color),
                        background_color: lane_color,
                        number_of_arrows: BigF32::new(
                            (start_anchor - end_anchor).length() / LANE_WIDTH,
                        ),
                        speeds: LaneShaderSpeeds {
                            forward: forward_speed_limit,
                            backward: backward_speed_limit,
                            ..Default::default()
                        },
                        bidirectional: BigU32::new(is_bidirectional as u32),
                        interacting: BigU32::new(false as u32),
                    },
                })),
                line_stroke_transform(&start_anchor, &end_anchor, LANE_WIDTH),
                Visibility::default(),
            ))
            .insert(ChildOf(rank_layer))
            .id();

        let mid_outline: Entity = commands
            .spawn((
                Mesh3d(assets.lane_mid_outline.clone()),
                MeshMaterial3d::<StandardMaterial>::default(),
                Transform::from_translation(-0.000_5 * Vec3::Z),
                Visibility::Hidden,
                ChildOf(mid),
                Pickable::IGNORE,
            ))
            .id();

        let z = ZLayer::Lane.to_z();
        commands.entity(e).insert((
            LaneSegments {
                rank_layer,
                start,
                mid,
                end,
                outlines: [start_outline, mid_outline, end_outline],
            },
            Transform::from_translation([0., 0., z].into()),
            visibility,
            Category::Lane,
            EdgeLabels::StartEnd,
        ));
    }
}

fn height_for_rank(rank: f32) -> f32 {
    rank * (ZLayer::Doormat.to_z() - ZLayer::Lane.to_z())
}

fn update_lane_visuals(
    entity: Entity,
    edge: &Edge<Entity>,
    segments: &LaneSegments,
    anchors: &AnchorParams,
    transforms: &mut Query<&mut Transform>,
    lane_materials: Query<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
    extended_materials: &mut ResMut<Assets<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
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

        if let Ok(mat) = lane_materials.get(segments.mid) {
            if let Some(lane_mat) = extended_materials.get_mut(&mat.0) {
                *lane_mat.extension.number_of_arrows =
                    (start_anchor - end_anchor).length() / LANE_WIDTH;
            }
        }
    }
    if let Some(mut tf) = transforms.get_mut(segments.end).ok() {
        *tf = Transform::from_translation(end_anchor);
    }
}

pub fn update_lane_motion_visuals(
    mut lanes: Query<
        (
            &LaneSegments,
            &Motion,
            &ReverseLane,
            Option<&RecallReverseLane>,
        ),
        Or<(Changed<Motion>, Changed<ReverseLane>, Changed<RecallMotion>)>,
    >,
    mut lane_materials: Query<
        &MeshMaterial3d<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>,
    >,
    mut extended_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
) {
    if lane_materials.is_empty() {
        return;
    }

    for (segments, motion, reverse, recall) in &mut lanes {
        if let Some(mat) = lane_materials.get_mut(segments.mid).ok() {
            if let Some(lane_mat) = extended_materials.get_mut(&mat.0) {
                let is_bidirectional = *reverse != ReverseLane::Disable;
                let forward_speed_limit = motion.speed_limit.unwrap_or(DEFAULT_LANE_ARROW_SPEED);
                let backward_speed_limit = match reverse {
                    ReverseLane::Same => forward_speed_limit,
                    _ => recall
                        .and_then(|rec| rec.previous.speed_limit)
                        .unwrap_or(DEFAULT_LANE_ARROW_SPEED),
                };

                lane_mat.extension.speeds.forward = forward_speed_limit;
                lane_mat.extension.speeds.backward = backward_speed_limit;
                *lane_mat.extension.bidirectional = is_bidirectional as u32;
            }
        }
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
    child_of: Query<&ChildOf>,
    levels: Query<(), With<LevelElevation>>,
    graphs: GraphSelect,
    mut transforms: Query<&mut Transform>,
    lane_materials: Query<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
    mut extended_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
    current_level: Res<CurrentLevel>,
) {
    for (e, edge, associated, segments, mut visibility) in &mut lanes {
        update_lane_visuals(
            e,
            edge,
            segments,
            &anchors,
            &mut transforms,
            lane_materials,
            &mut extended_materials,
        );

        let new_visibility = if should_display_lane(
            edge,
            associated,
            &child_of,
            &levels,
            &current_level,
            &graphs,
        ) {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
        if *visibility != new_visibility {
            *visibility = new_visibility;
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
    lane_materials: Query<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
    mut extended_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
) {
    for dependents in &changed_anchors {
        for dependent in dependents.iter() {
            if let Ok((e, edge, segments)) = lanes.get(*dependent) {
                update_lane_visuals(
                    e,
                    edge,
                    segments,
                    &anchors,
                    &mut transforms,
                    lane_materials,
                    &mut extended_materials,
                );
            }
        }
    }
}

pub fn remove_association_for_deleted_graphs(
    mut associaged_graphs: Query<&mut AssociatedGraphs<Entity>>,
    mut removed: RemovedComponents<NavGraphMarker>,
) {
    for e in removed.read() {
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

pub fn update_color_for_lanes(
    changed_lanes: Query<
        (&AssociatedGraphs<Entity>, &LaneSegments),
        (With<LaneMarker>, Changed<AssociatedGraphs<Entity>>),
    >,
    any_graphs_changed: Query<
        (),
        (
            Or<(
                Changed<DisplayColor>,
                Changed<RecencyRank<NavGraphMarker>>,
                Changed<Visibility>,
            )>,
            With<NavGraphMarker>,
        ),
    >,
    all_lanes: Query<(&AssociatedGraphs<Entity>, &LaneSegments), With<LaneMarker>>,
    graphs: GraphSelect,
    lane_materials: Query<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
    mut extended_materials: ResMut<Assets<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
) {
    if any_graphs_changed.is_empty() {
        // No nav graph colors have changed, so only look at lanes who have changed
        // their associated graphs
        for (associated, segments) in changed_lanes {
            impl_update_color_for_lane(
                associated,
                segments,
                &graphs,
                &lane_materials,
                &mut extended_materials,
            );
        }
    } else {
        // A nav graph color has changed, so update all lanes just to be safe
        for (associated, segments) in all_lanes {
            impl_update_color_for_lane(
                associated,
                segments,
                &graphs,
                &lane_materials,
                &mut extended_materials,
            );
        }
    }
}

fn impl_update_color_for_lane(
    associated: &AssociatedGraphs<Entity>,
    segments: &LaneSegments,
    graphs: &GraphSelect,
    lane_materials: &Query<&MeshMaterial3d<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
    extended_materials: &mut ResMut<Assets<ExtendedMaterial<StandardMaterial, LaneArrowMaterial>>>,
) {
    let (_, color, _) = graphs.display_style(associated);
    let new_color = color.to_linear();

    if let Ok(ext_mat) = lane_materials.get(segments.mid) {
        if let Some(lane_mat) = extended_materials.get_mut(&ext_mat.0) {
            lane_mat.extension.background_color = new_color;
            lane_mat.extension.single_arrow_color = forward_arrow_color(new_color);
            lane_mat.extension.double_arrow_color = backward_arrow_color(new_color);
        }
    }
}

fn forward_arrow_color(lane_color: LinearRgba) -> LinearRgba {
    let dark_color_diff = 0.1;
    Color::srgb(
        (lane_color.red - dark_color_diff).clamp(0.0, 1.0),
        (lane_color.green - dark_color_diff).clamp(0.0, 1.0),
        (lane_color.blue - dark_color_diff).clamp(0.0, 1.0),
    )
    .into()
}

fn backward_arrow_color(lane_color: LinearRgba) -> LinearRgba {
    let light_color_diff = 0.5;
    Color::srgb(
        (lane_color.red + light_color_diff).clamp(0.0, 1.0),
        (lane_color.green + light_color_diff).clamp(0.0, 1.0),
        (lane_color.blue + light_color_diff).clamp(0.0, 1.0),
    )
    .into()
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
    child_of: Query<&ChildOf>,
    levels: Query<(), With<LevelElevation>>,
    current_level: Res<CurrentLevel>,
    graphs: GraphSelect,
    lanes_with_changed_association: Query<
        (Entity, &AssociatedGraphs<Entity>, &LaneSegments),
        (With<LaneMarker>, Changed<AssociatedGraphs<Entity>>),
    >,
    mut materials: Query<&mut MeshMaterial3d<StandardMaterial>, Without<NavGraphMarker>>,
    mut transforms: Query<&mut Transform>,
    graph_changed_visibility: Query<
        (),
        (
            With<NavGraphMarker>,
            Or<(Changed<Visibility>, Changed<RecencyRank<NavGraphMarker>>)>,
        ),
    >,
    mut removed: RemovedComponents<NavGraphMarker>,
) {
    let graph_change = !graph_changed_visibility.is_empty() || removed.read().next().is_some();
    let update_all = current_level.is_changed() || graph_change;
    if update_all {
        for (edge, associated, _, mut visibility) in &mut lanes {
            let new_visibility = if should_display_lane(
                edge,
                associated,
                &child_of,
                &levels,
                &current_level,
                &graphs,
            ) {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
            if *visibility != new_visibility {
                *visibility = new_visibility;
            }
        }
    } else {
        for (e, _, _) in &lanes_with_changed_association {
            if let Ok((edge, associated, _, mut visibility)) = lanes.get_mut(e) {
                let new_visibility = if should_display_lane(
                    edge,
                    associated,
                    &child_of,
                    &levels,
                    &current_level,
                    &graphs,
                ) {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                if *visibility != new_visibility {
                    *visibility = new_visibility;
                }
            }
        }
    }

    if graph_change {
        for (_, associated_graphs, segments, _) in &lanes {
            let (mat, _, rank) = graphs.display_style(associated_graphs);
            for e in segments.iter() {
                if let Ok(mut m) = materials.get_mut(e) {
                    *m = MeshMaterial3d(mat.clone());
                }
            }

            if let Ok(mut tf) = transforms.get_mut(segments.rank_layer) {
                tf.translation.z = height_for_rank(rank);
            }
        }
    } else {
        for (_, associated_graphs, segments) in &lanes_with_changed_association {
            let (mat, _, rank) = graphs.display_style(associated_graphs);
            for e in segments.iter() {
                if let Ok(mut m) = materials.get_mut(e) {
                    *m = MeshMaterial3d(mat.clone());
                }
            }

            if let Ok(mut tf) = transforms.get_mut(segments.rank_layer) {
                tf.translation.z = height_for_rank(rank);
            }
        }
    }
}

#[derive(Debug, Clone, Copy, Event)]
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
    for consider in considerations.read() {
        if let Ok(mut recall) = recalls.get_mut(consider.for_element) {
            recall.consider = consider.graph;
        }
    }
}

/// Unique UUID to identify issue of duplicated dock names
pub const DUPLICATED_DOCK_NAME_ISSUE_UUID: Uuid =
    Uuid::from_u128(0xca210e1025014ac2a4072bc956d76151u128);

// When triggered by a validation request event, check if there are duplicated dock names and
// generate an issue if that is the case
pub fn check_for_duplicated_dock_names(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    child_of: Query<&ChildOf>,
    lane_properties: Query<(Entity, &Motion, Option<&ReverseLane>), With<LaneMarker>>,
) {
    const ISSUE_HINT: &str = "RMF uses the dock name parameter to trigger special behavior from \
                        the robots. Duplicated dock names would make such behavior ambiguous as \
                        it would be triggered in different parts of the map, rename the docks to \
                        be unique";
    for root in validate_events.read() {
        let mut names: HashMap<String, BTreeSet<Entity>> = HashMap::new();
        for (e, motion, reverse) in &lane_properties {
            if AncestorIter::new(&child_of, e).any(|co| co == **root) {
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
                                hint: ISSUE_HINT.to_string(),
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
                    hint: ISSUE_HINT.to_string(),
                };
                let id = commands.spawn(issue).id();
                commands.entity(**root).add_child(id);
            }
        }
    }
}
