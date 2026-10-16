/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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

use bevy::ecs::hierarchy::ChildOf;
use bevy::prelude::*;
use rmf_site_egui::InspectFor;
use rmf_site_format::{
    Anchor, AssociatedGraphs, FloorMarker, Group, LaneMarker, LevelElevation, LocationTags,
    NameInSite, NavGraphMarker, Pending, Point, Pose, Robot, SiteID,
};
use rmf_site_msgs::rmf_prototype_msgs::msg::{GraphElementKey, TargetNode};
use rmf_site_picking::{CursorFrame, Hovering, PickBlockStatus, Select, SelectionServiceStages};
use std::collections::HashMap;

use crate::layers::ZLayer;
use crate::site::{LocationBillboardMarker, LANE_WIDTH};

use super::gizmos::{draw_crosshair_symbol, draw_dashed_line, draw_off_limits_symbol};
use super::live_state::{LiveStreamState, LoadSiteStatus};

const DESTINATION_TARGET_OUTER_RADIUS: f32 = 0.06;
const DESTINATION_TARGET_INNER_RADIUS: f32 = 0.02;
const DESTINATION_TARGET_CROSS_RADIUS: f32 = 0.1;
const DESTINATION_TARGET_COLOR: Color = Color::srgb(1.0, 0.0, 0.0);

const DESTINATION_LINE_DASH: f32 = 0.2;
const DESTINATION_LINE_GAP: f32 = 0.2;

// TODO: As locations currently use a torus mesh, if the user clicks within the center hole we cannot
// use hover/select logic to determine if it is part of the location. A temporary fix is to find the
// radius of the location toruses and check if the click is within the range of any locations. We should
// find a way to more concretely do this instead of relying on distance checking.
const LOCATION_OUTER_RADIUS: f32 = 1.4 * LANE_WIDTH / 2.0;

#[derive(Default)]
pub struct DestinationPickingPlugin {}

impl Plugin for DestinationPickingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DestinationPickingState>().add_systems(
            Update,
            (select_robot_destination, draw_robot_destination_visuals)
                .chain()
                .after(SelectionServiceStages::Hover)
                .before(SelectionServiceStages::Select)
                .run_if(in_state(crate::AppState::SiteStream))
                .run_if(LiveStreamState::in_connected_mode()),
        );
    }
}

#[derive(Clone, Debug)]
pub struct RobotDestination {
    pub position: Vec2,
    pub location_name: Option<String>,
    pub target_nodes: Vec<TargetNode>,
}

impl RobotDestination {
    pub fn display_label(&self) -> String {
        self.location_name
            .clone()
            .unwrap_or_else(|| format!("({:.2}, {:.2})", self.position.x, self.position.y))
    }
}

#[derive(Default, Resource)]
pub struct DestinationPickingState {
    pub active_routing_robot: Option<String>,
    pub selected_destinations: HashMap<String, RobotDestination>,
}

impl DestinationPickingState {
    pub fn is_empty(&self) -> bool {
        self.active_routing_robot.is_none() && self.selected_destinations.is_empty()
    }
}

fn select_robot_destination(
    mut picking_state: ResMut<DestinationPickingState>,
    block_status: Res<PickBlockStatus>,
    buttons: Res<ButtonInput<MouseButton>>,
    mut select_events: ResMut<Events<Select>>,
    cursor: Query<&Transform, With<CursorFrame>>,
    hovering: Res<Hovering>,
    ground: Query<
        (),
        Or<(
            With<FloorMarker>,
            With<LaneMarker>,
            With<LocationTags>,
            With<LocationBillboardMarker>,
            With<Anchor>,
        )>,
    >,
    inspect_for: Query<&InspectFor>,
    locations: Query<
        (
            Entity,
            &NameInSite,
            &Point<Entity>,
            &GlobalTransform,
            &InheritedVisibility,
            Option<&SiteID>,
            &AssociatedGraphs<Entity>,
        ),
        (With<LocationTags>, Without<Pending>),
    >,
    anchors: Query<(Option<&SiteID>, Option<&ChildOf>), With<Anchor>>,
    levels: Query<&NameInSite, With<LevelElevation>>,
    nav_graphs: Query<(Entity, &NameInSite), (With<NavGraphMarker>, Without<Pending>)>,
) {
    let Some(active_robot) = picking_state.active_routing_robot.clone() else {
        return;
    };

    if !buttons.just_pressed(MouseButton::Left) || block_status.blocked() {
        return;
    }

    select_events.clear();

    let Ok(cursor_transform) = cursor.single() else {
        return;
    };
    let point = cursor_transform.translation;

    // Verify that destination is on the ground
    let pointing_at_ground = hovering.0.is_some_and(|e| ground.contains(e));

    if !pointing_at_ground {
        warn!("Invalid destination");
        return;
    }

    let cursor_xy = Vec2::new(point.x, point.y);
    let hovered_entity = hovering
        .0
        .map(|e| inspect_for.get(e).map_or(e, |i| i.entity));

    // If destination is within a location, snap to location center
    let matched_location = locations
        .iter()
        .filter(|(_, _, _, _, vis, _, _)| vis.get())
        .filter_map(
            |(loc_entity, name, loc_point, loc_tf, _, loc_site_id, associated_graphs)| {
                let loc_xy = loc_tf.translation().truncate();
                let dist = cursor_xy.distance(loc_xy);
                let directly_hovered =
                    hovered_entity == Some(loc_entity) || hovered_entity == Some(loc_point.0);
                if directly_hovered || dist <= LOCATION_OUTER_RADIUS {
                    let effective_dist = if directly_hovered { 0.0 } else { dist };
                    Some((
                        effective_dist,
                        loc_xy,
                        &name.0,
                        loc_point.0,
                        loc_site_id.map(|s| s.0),
                        associated_graphs,
                    ))
                } else {
                    None
                }
            },
        )
        .min_by(|a, b| a.0.total_cmp(&b.0));

    // Build the destination and attach graph node data if snapped to a named location
    let destination =
        if let Some((_, loc_center, loc_name, anchor_entity, loc_site_id, associated_graphs)) =
            matched_location
        {
            let anchor = anchors.get(anchor_entity).ok();

            let vertex_id = loc_site_id
                .or_else(|| anchor.and_then(|(site_id, _)| site_id.map(|s| s.0)))
                .map(|id| id as i64);

            let level_name = anchor
                .and_then(|(_, child_of)| child_of)
                .and_then(|co| levels.get(co.parent()).ok())
                .map(|n| n.0.clone());

            let target_nodes = nav_graphs
                .iter()
                .filter(|(graph_entity, _)| associated_graphs.includes(*graph_entity))
                .map(|(_, graph_name)| TargetNode {
                    key: GraphElementKey {
                        graph_name: graph_name.0.clone(),
                        key: vertex_id.into_iter().collect(),
                        name: vec![loc_name.clone()],
                        map: level_name.clone().into_iter().collect(),
                        element_type: GraphElementKey::ELEMENT_TYPE_NODE,
                    },
                    orientations: Vec::new(),
                })
                .collect();

            RobotDestination {
                position: loc_center,
                location_name: Some(loc_name.clone()),
                target_nodes,
            }
        } else {
            // Free-space click. Use raw cursor coordinates with no associated graph nodes
            RobotDestination {
                position: cursor_xy,
                location_name: None,
                target_nodes: Vec::new(),
            }
        };

    info!(
        "[Destination Select] robot={} cursor=({:.2}, {:.2}) matched_location={:?} target_nodes={:?} final_dest=({:.2}, {:.2})",
        active_robot,
        cursor_xy.x,
        cursor_xy.y,
        destination.location_name,
        destination.target_nodes,
        destination.position.x,
        destination.position.y,
    );

    picking_state
        .selected_destinations
        .insert(active_robot, destination);
    picking_state.active_routing_robot = None;
}

fn draw_robot_destination_visuals(
    state: Res<LiveStreamState>,
    picking_state: Res<DestinationPickingState>,
    mut gizmos: Gizmos,
    cursor: Query<&Transform, With<CursorFrame>>,
    hovering: Res<Hovering>,
    block_status: Res<PickBlockStatus>,
    ground: Query<
        (),
        Or<(
            With<FloorMarker>,
            With<LaneMarker>,
            With<LocationTags>,
            With<LocationBillboardMarker>,
            With<Anchor>,
        )>,
    >,
    robots: Query<(&NameInSite, &Pose), (With<Robot>, Without<Group>, Without<Pending>)>,
) {
    if state.load_site_status != LoadSiteStatus::None {
        return;
    }

    let z_height = ZLayer::DestinationTarget.to_z();

    // Draw visuals for already-selected destinations
    for (robot_name, selected_dest) in &picking_state.selected_destinations {
        if picking_state.active_routing_robot.as_deref() == Some(robot_name.as_str()) {
            continue;
        }
        let target_pos = Vec3::new(selected_dest.position.x, selected_dest.position.y, z_height);
        draw_crosshair_symbol(
            &mut gizmos,
            target_pos,
            Quat::IDENTITY,
            DESTINATION_TARGET_OUTER_RADIUS,
            DESTINATION_TARGET_INNER_RADIUS,
            DESTINATION_TARGET_CROSS_RADIUS,
            DESTINATION_TARGET_COLOR,
        );
        if let Some((_, pose)) = robots.iter().find(|(name, _)| &name.0 == robot_name) {
            draw_dashed_line(
                &mut gizmos,
                Vec3::new(pose.trans[0], pose.trans[1], z_height),
                target_pos,
                DESTINATION_LINE_DASH,
                DESTINATION_LINE_GAP,
                0.0,
                DESTINATION_TARGET_COLOR,
            );
        }
    }

    // Draw visuals for the actively routing robot, if any
    let Some(active_robot) = &picking_state.active_routing_robot else {
        return;
    };
    if block_status.blocked() {
        return;
    }
    let Ok(cursor_transform) = cursor.single() else {
        return;
    };

    let cursor_pos = cursor_transform.translation.with_z(z_height);
    let pointing_at_ground = hovering.0.is_some_and(|e| ground.contains(e));
    if !pointing_at_ground {
        draw_off_limits_symbol(
            &mut gizmos,
            cursor_pos,
            cursor_transform.rotation,
            DESTINATION_TARGET_OUTER_RADIUS,
            DESTINATION_TARGET_COLOR,
        );
        return;
    }

    draw_crosshair_symbol(
        &mut gizmos,
        cursor_pos,
        Quat::IDENTITY,
        DESTINATION_TARGET_OUTER_RADIUS,
        DESTINATION_TARGET_INNER_RADIUS,
        DESTINATION_TARGET_CROSS_RADIUS,
        DESTINATION_TARGET_COLOR,
    );
    if let Some((_, pose)) = robots.iter().find(|(name, _)| &name.0 == active_robot) {
        draw_dashed_line(
            &mut gizmos,
            Vec3::new(pose.trans[0], pose.trans[1], z_height),
            cursor_pos,
            DESTINATION_LINE_DASH,
            DESTINATION_LINE_GAP,
            0.0,
            DESTINATION_TARGET_COLOR,
        );
    }
}
