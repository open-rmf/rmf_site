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

use bevy::prelude::*;
use rmf_site_msgs::rmf_prototype_msgs::msg::{Plan, Progress};
use roslibrust::rosbridge::ClientHandle;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use crate::layers::ZLayer;

use super::network_client::{
    run_subscription_loop, spawn_network_task, LiveStreamHandler, VisualizationStreamChannel,
};
use super::odometry::LiveRobotsState;

const PLANNED_PATH_COLOR: Color = Color::srgb(0.0, 1.0, 0.0);

const DEPENDENCY_LINE_COLOR: Color = Color::srgb(1.0, 0.5, 0.0);
const DEPENDENCY_WAITING_POINT_COLOR: Color = Color::srgb(1.0, 0.85, 0.0);
const DEPENDENCY_DASH_LENGTH: f32 = 0.15;
const DEPENDENCY_GAP_LENGTH: f32 = 0.1;
const DEPENDENCY_LINE_SPEED: f32 = 0.5;
const DEPENDENCY_ARROW_SIZE: f32 = 0.1;

const PATH_POINT_OUTER_RADIUS: f32 = 0.05;
const PATH_POINT_INNER_RADIUS: f32 = 0.02;
const PATH_POINT_CROSS_RADIUS: f32 = 0.02;

#[derive(Debug, Clone, PartialEq)]
struct LiveBlocker {
    name: String,
    required_progress: f32,
}

#[derive(Debug, Clone, PartialEq)]
struct LiveWaypoint {
    position: Vec3,
    progress: f32,
    departure_blockers: Vec<LiveBlocker>,
}

#[derive(Debug, Clone)]
pub struct LiveEventPlan {
    name: String,
    waypoints: Vec<LiveWaypoint>,
}

impl LiveStreamHandler for LiveEventPlan {
    fn spawn_stream(
        robot_name: String,
        client: ClientHandle,
        sender: UnboundedSender<Self>,
        connection_requested: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    ) {
        let topic_name = format!("/{}/plan", robot_name);

        let task = async move {
            if let Ok(plan_sub) = client.subscribe_transient_local::<Plan>(&topic_name).await {
                run_subscription_loop(
                    plan_sub,
                    sender,
                    connection_requested,
                    connection_active,
                    |plan_msg| {
                        let waypoints: Vec<LiveWaypoint> = plan_msg
                            .waypoints
                            .iter()
                            .map(|wp| {
                                let blockers = wp
                                    .departure_blockers
                                    .iter()
                                    .map(|b| LiveBlocker {
                                        name: b.name.clone(),
                                        required_progress: b.required_progress,
                                    })
                                    .collect();

                                LiveWaypoint {
                                    position: Vec3::new(
                                        wp.position[0],
                                        wp.position[1],
                                        ZLayer::PlannedPath.to_z(),
                                    ),
                                    progress: wp.progress,
                                    departure_blockers: blockers,
                                }
                            })
                            .collect();

                        LiveEventPlan {
                            name: robot_name.clone(),
                            waypoints,
                        }
                    },
                )
                .await;
            }
        };
        spawn_network_task(task);
    }

    fn cleanup(world: &mut World) {
        if world
            .get_resource::<LivePathsState>()
            .is_none_or(|s| s.0.is_empty())
        {
            return;
        }
        let _ = world.run_system_cached(|mut path_state: ResMut<LivePathsState>| {
            path_state.0.clear();
        });
    }
}

#[derive(Debug, Clone)]
pub struct LiveEventProgress {
    name: String,
    reached_waypoint: usize,
    target_waypoint: usize,
    progress: f32,
}

impl LiveStreamHandler for LiveEventProgress {
    fn spawn_stream(
        robot_name: String,
        client: ClientHandle,
        sender: UnboundedSender<Self>,
        connection_requested: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    ) {
        let topic_name = format!("/{}/plan/progress", robot_name);

        let task = async move {
            if let Ok(prog_sub) = client
                .subscribe_transient_local::<Progress>(&topic_name)
                .await
            {
                run_subscription_loop(
                    prog_sub,
                    sender,
                    connection_requested,
                    connection_active,
                    |prog_msg| LiveEventProgress {
                        name: robot_name.clone(),
                        reached_waypoint: prog_msg.reached_waypoint as usize,
                        target_waypoint: prog_msg.target_waypoint as usize,
                        progress: prog_msg.progress,
                    },
                )
                .await;
            }
        };
        spawn_network_task(task);
    }
}

#[derive(Default, Resource)]
pub struct LivePathsState(pub HashMap<String, PlannedPathData>);

pub struct PlannedPathData {
    waypoints: Vec<LiveWaypoint>,
    target_waypoint: Option<usize>,
    completed_waypoint: usize,
    current_progress: f32,
}

impl PlannedPathData {
    pub fn is_completed(&self) -> bool {
        self.waypoints.is_empty()
            || self
                .waypoints
                .last()
                .is_some_and(|last_wp| self.current_progress >= last_wp.progress)
    }
}

pub fn update_live_paths(
    time: Res<Time>,
    mut plan_channel: ResMut<VisualizationStreamChannel<LiveEventPlan>>,
    mut progress_channel: ResMut<VisualizationStreamChannel<LiveEventProgress>>,
    mut path_state: ResMut<LivePathsState>,
    robot_map: Res<LiveRobotsState>,
    mut gizmos: Gizmos,
) {
    while let Ok(event) = plan_channel.receiver.try_recv() {
        let robot_path = path_state
            .0
            .entry(event.name.clone())
            .or_insert(PlannedPathData {
                waypoints: Vec::new(),
                target_waypoint: None,
                completed_waypoint: 0,
                current_progress: f32::MAX,
            });

        if robot_path.waypoints != event.waypoints {
            // Check if this is a detour/new path
            let has_existing_path = !robot_path.waypoints.is_empty();
            robot_path.waypoints = event.waypoints;

            // If the robot already had a path, this is a brand new detour.
            // Target and completed waypoint are reset to the beginning of the new path.
            // If it did not have an existing path, the Progress message arrived
            // first, so the target_waypoint and completed_waypoint are left alone.
            if has_existing_path {
                robot_path.target_waypoint = None;
                robot_path.completed_waypoint = 0;
                robot_path.current_progress = 0.0;
            }
        }
    }

    while let Ok(event) = progress_channel.receiver.try_recv() {
        let robot_path = path_state
            .0
            .entry(event.name.clone())
            .or_insert(PlannedPathData {
                waypoints: Vec::new(),
                target_waypoint: Some(event.target_waypoint),
                completed_waypoint: 0,
                current_progress: event.progress,
            });

        if let Some(prev_target) = robot_path.target_waypoint {
            if event.target_waypoint > prev_target {
                // Previous incremental target was just completed, update completed waypoint
                robot_path.completed_waypoint = prev_target.min(event.reached_waypoint);
            } else if event.target_waypoint < prev_target {
                // New path started, reset completed waypoing
                robot_path.completed_waypoint = 0;
            }
        }

        robot_path.target_waypoint = Some(event.target_waypoint);
        robot_path.current_progress = event.progress;
    }

    for (name, path_data) in path_state.0.iter() {
        if path_data.is_completed() || !robot_map.0.contains_key(name) {
            continue;
        }

        let start_idx = path_data
            .completed_waypoint
            .min(path_data.waypoints.len().saturating_sub(1));

        // Draw line from the last completed incremental target waypoint along the path to the final waypoint.
        if start_idx < path_data.waypoints.len() {
            let points_to_draw: Vec<Vec3> = path_data.waypoints[start_idx..]
                .iter()
                .map(|wp| wp.position)
                .collect();

            if points_to_draw.len() > 1 {
                gizmos.linestrip(points_to_draw, PLANNED_PATH_COLOR);

                draw_path_start_point(
                    &mut gizmos,
                    path_data.waypoints[start_idx].position,
                    PLANNED_PATH_COLOR,
                );

                if let Some(final_wp) = path_data.waypoints.last() {
                    draw_path_end_point(&mut gizmos, final_wp.position, PLANNED_PATH_COLOR);
                }
            }
        }

        for (i, wp) in path_data.waypoints.iter().enumerate() {
            // Disappear if the waiting robot has already passed this waypoint
            if path_data.current_progress >= wp.progress {
                continue;
            }

            for blocker in &wp.departure_blockers {
                if let Some(blocking_path) = path_state.0.get(&blocker.name) {
                    // Skip drawing if the dependency is fulfilled
                    if blocking_path.current_progress >= blocker.required_progress {
                        continue;
                    }

                    // Find the coordinates where the blocking robot will clear the dependency
                    let mut clearance_pos = None;
                    for blocking_wp in &blocking_path.waypoints {
                        if blocking_wp.progress >= blocker.required_progress {
                            clearance_pos = Some(Vec3::new(
                                blocking_wp.position.x,
                                blocking_wp.position.y,
                                ZLayer::PlannedPathPoint.to_z(),
                            ));
                            break;
                        }
                    }

                    // Draw dependency line connecting the waiting point to the clearance point
                    if let Some(end_pos) = clearance_pos {
                        let start_pos = Vec3::new(
                            wp.position.x,
                            wp.position.y,
                            ZLayer::PlannedPathPoint.to_z(),
                        );
                        let wait_wp = if i > 0 {
                            &path_data.waypoints[i - 1]
                        } else {
                            wp
                        };
                        let waiting_pos = Vec3::new(
                            wait_wp.position.x,
                            wait_wp.position.y,
                            ZLayer::PlannedPathPoint.to_z(),
                        );
                        draw_dependency_line(start_pos, end_pos, waiting_pos, &time, &mut gizmos);
                    }
                }
            }
        }
    }
}

fn draw_dependency_line(
    start_pos: Vec3,
    end_pos: Vec3,
    waiting_pos: Vec3,
    time: &Time,
    gizmos: &mut Gizmos,
) {
    let delta = end_pos - start_pos;
    let distance = delta.length();

    if distance > 0.0 {
        let dir = delta / distance;
        let pattern_length = DEPENDENCY_DASH_LENGTH + DEPENDENCY_GAP_LENGTH;
        let offset = (time.elapsed_secs() * DEPENDENCY_LINE_SPEED) % pattern_length;
        let mut current_dist = offset - pattern_length;

        // Draw dashed line along vector
        while current_dist < distance {
            let start_dist = current_dist.max(0.0);
            let end_dist = (current_dist + DEPENDENCY_DASH_LENGTH).min(distance);

            if start_dist < end_dist {
                let segment_start = start_pos + dir * start_dist;
                let segment_end = start_pos + dir * end_dist;

                gizmos.line(segment_start, segment_end, DEPENDENCY_LINE_COLOR);
            }

            current_dist += pattern_length;
        }

        draw_path_waiting_point(gizmos, waiting_pos, DEPENDENCY_WAITING_POINT_COLOR);
        draw_path_arrowhead(gizmos, end_pos, dir, DEPENDENCY_LINE_COLOR);
    }
}

fn draw_path_arrowhead(gizmos: &mut Gizmos, pos: Vec3, dir: Vec3, color: Color) {
    let perp = Vec3::new(-dir.y, dir.x, 0.0);
    let p1 = pos - dir * DEPENDENCY_ARROW_SIZE + perp * (DEPENDENCY_ARROW_SIZE * 0.5);
    let p2 = pos - dir * DEPENDENCY_ARROW_SIZE - perp * (DEPENDENCY_ARROW_SIZE * 0.5);

    gizmos.line(pos, p1, color);
    gizmos.line(pos, p2, color);
}

fn draw_path_waiting_point(gizmos: &mut Gizmos, pos: Vec3, color: Color) {
    let new_pos = Isometry3d::new(pos.with_z(ZLayer::PlannedPathPoint.to_z()), Quat::IDENTITY);
    gizmos.circle(new_pos, PATH_POINT_OUTER_RADIUS, color);
    gizmos.circle(new_pos, PATH_POINT_INNER_RADIUS, color);
}

fn draw_path_start_point(gizmos: &mut Gizmos, pos: Vec3, color: Color) {
    let new_pos = pos.with_z(ZLayer::PlannedPathPoint.to_z());
    let isometry_pos = Isometry3d::new(pos.with_z(ZLayer::PlannedPathPoint.to_z()), Quat::IDENTITY);
    gizmos.circle(isometry_pos, PATH_POINT_INNER_RADIUS, color);
    gizmos.line(
        new_pos + Vec3::X * PATH_POINT_CROSS_RADIUS,
        new_pos - Vec3::X * PATH_POINT_CROSS_RADIUS,
        color,
    );
    gizmos.line(
        new_pos + Vec3::Y * PATH_POINT_CROSS_RADIUS,
        new_pos - Vec3::Y * PATH_POINT_CROSS_RADIUS,
        color,
    );
}

fn draw_path_end_point(gizmos: &mut Gizmos, pos: Vec3, color: Color) {
    let new_pos = pos.with_z(ZLayer::PlannedPathPoint.to_z());
    let isometry_pos = Isometry3d::new(new_pos, Quat::IDENTITY);
    gizmos.circle(isometry_pos, PATH_POINT_OUTER_RADIUS, color);
    gizmos.circle(isometry_pos, PATH_POINT_INNER_RADIUS, color);
    gizmos.line(
        new_pos + Vec3::X * PATH_POINT_CROSS_RADIUS,
        new_pos - Vec3::X * PATH_POINT_CROSS_RADIUS,
        color,
    );
    gizmos.line(
        new_pos + Vec3::Y * PATH_POINT_CROSS_RADIUS,
        new_pos - Vec3::Y * PATH_POINT_CROSS_RADIUS,
        color,
    );
}
