use bevy::prelude::*;
use rmf_site_msgs::rmf_prototype_msgs::msg::{Plan, Progress};
use roslibrust::rosbridge::ClientHandle;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use super::live_state::LiveStreamState;
use super::network_client::{
    spawn_network_task, wait_until_inactive, LiveStreamHandler, VisualizationStreamChannel,
};
use super::odometry::{LiveRobotMarker, LiveRobotsMap};

const PLANNED_PATH_Z_OFFSET: f32 = 0.05;
const PLANNED_PATH_COLOR: Color = Color::srgb(0.0, 1.0, 0.0);

const DEPENDENCY_Z_OFFSET: f32 = 0.051;
const DEPENDENCY_LINE_COLOR: Color = Color::srgb(1.0, 0.5, 0.0);
const DEPENDENCY_WAITING_POINT_COLOR: Color = Color::srgb(1.0, 0.85, 0.0);
const DEPENDENCY_DASH_LENGTH: f32 = 0.15;
const DEPENDENCY_GAP_LENGTH: f32 = 0.1;
const DEPENDENCY_LINE_SPEED: f32 = 0.5;
const DEPENDENCY_ARROW_SIZE: f32 = 0.1;

const PATH_POINT_OUTER_RADIUS: f32 = 0.05;
const PATH_POINT_INNER_RADIUS: f32 = 0.02;

const PATH_ENDPOINT_COLOR: Color = Color::srgb(1.0, 0.0, 0.0);
const PATH_ENDPOINT_CROSS_RADIUS: f32 = 0.1;
const PATH_ENDPOINT_Z_OFFSET: f32 = 0.001;

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
        connect_flag: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    ) {
        let topic_name = format!("/{}/plan", robot_name);

        let task = async move {
            if let Ok(plan_sub) = client.subscribe_transient_local::<Plan>(&topic_name).await {
                loop {
                    let plan_msg = tokio::select! {
                        msg = plan_sub.next() => msg,
                        _ = wait_until_inactive(&connection_active) => break,
                    };

                    if !connect_flag.load(Ordering::Relaxed)
                        || !connection_active.load(Ordering::Relaxed)
                    {
                        break;
                    }

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
                                    wp.position[0] as f32,
                                    wp.position[1] as f32,
                                    PLANNED_PATH_Z_OFFSET,
                                ),
                                progress: wp.progress,
                                departure_blockers: blockers,
                            }
                        })
                        .collect();

                    if let Err(e) = sender.send(LiveEventPlan {
                        name: robot_name.clone(),
                        waypoints,
                    }) {
                        error!("Failed to send Plan event across channel: {}", e);
                        break;
                    }
                }
            }
        };
        spawn_network_task(task);
    }
}

#[derive(Debug, Clone)]
pub struct LiveEventProgress {
    name: String,
    target_waypoint: usize,
    progress: f32,
}

impl LiveStreamHandler for LiveEventProgress {
    fn spawn_stream(
        robot_name: String,
        client: ClientHandle,
        sender: UnboundedSender<Self>,
        connect_flag: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    ) {
        let topic_name = format!("/{}/plan/progress", robot_name);

        let task = async move {
            if let Ok(prog_sub) = client
                .subscribe_transient_local::<Progress>(&topic_name)
                .await
            {
                loop {
                    let prog_msg = tokio::select! {
                        msg = prog_sub.next() => msg,
                        _ = wait_until_inactive(&connection_active) => break,
                    };

                    if !connect_flag.load(Ordering::Relaxed)
                        || !connection_active.load(Ordering::Relaxed)
                    {
                        break;
                    }

                    if let Err(e) = sender.send(LiveEventProgress {
                        name: robot_name.clone(),
                        target_waypoint: prog_msg.target_waypoint as usize,
                        progress: prog_msg.progress,
                    }) {
                        error!("Failed to send Progress event across channel: {}", e);
                        break;
                    }
                }
            }
        };
        spawn_network_task(task);
    }
}

#[derive(Default, Resource)]
pub struct LivePathsState(pub HashMap<String, PlannedPathData>);

pub struct PlannedPathData {
    waypoints: Vec<LiveWaypoint>,
    target_waypoint: usize,
    current_progress: f32,
}

impl PlannedPathData {
    pub fn is_completed(&self) -> bool {
        self.waypoints.is_empty()
            || self.target_waypoint >= self.waypoints.len()
            || self
                .waypoints
                .last()
                .is_some_and(|last_wp| self.current_progress >= last_wp.progress)
    }
}

pub fn update_live_paths(
    state: Res<LiveStreamState>,
    time: Res<Time>,
    mut plan_channel: ResMut<VisualizationStreamChannel<LiveEventPlan>>,
    mut progress_channel: ResMut<VisualizationStreamChannel<LiveEventProgress>>,
    mut path_state: ResMut<LivePathsState>,
    robot_map: Res<LiveRobotsMap>,
    robot_query: Query<&Transform, With<LiveRobotMarker>>,
    mut gizmos: Gizmos,
) {
    if !state.connection_active.load(Ordering::Relaxed) {
        path_state.0.clear();
        return;
    }

    while let Ok(event) = plan_channel.receiver.try_recv() {
        let robot_path = path_state
            .0
            .entry(event.name.clone())
            .or_insert(PlannedPathData {
                waypoints: Vec::new(),
                target_waypoint: 1,
                current_progress: f32::MAX,
            });

        if robot_path.waypoints != event.waypoints {
            // Check if this is a detour/new path
            let has_existing_path = !robot_path.waypoints.is_empty();
            robot_path.waypoints = event.waypoints;

            // If the robot already had a path, this is a brand new detour.
            // Target is reset to the beginning of the new path.
            // If it did not have an existing path, the Progress message arrived
            // first, so the target_waypoint is left alone.
            if has_existing_path {
                robot_path.target_waypoint = 1;
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
                target_waypoint: event.target_waypoint,
                current_progress: event.progress,
            });
        robot_path.target_waypoint = event.target_waypoint;
        robot_path.current_progress = event.progress;
    }

    for (name, path_data) in path_state.0.iter() {
        if path_data.is_completed() {
            continue;
        }

        let mut robot_pos = None;
        if let Some(&robot_entity) = robot_map.0.get(name) {
            if let Ok(transform) = robot_query.get(robot_entity) {
                robot_pos = Some(Vec3::new(
                    transform.translation.x,
                    transform.translation.y,
                    PLANNED_PATH_Z_OFFSET,
                ));
            }
        }

        let start_pos = match robot_pos {
            Some(pos) => pos,
            None => continue,
        };

        let mut final_target_idx = path_data
            .target_waypoint
            .min(path_data.waypoints.len().saturating_sub(1));

        // Handle bug where target waypoint is prematurely updated, causing the robot to skip intermediate waypoints.
        // Compare the progress of each waypoint with the current progress to get the true unreached waypoint.
        // This loop likely only needs to check the current and previous waypoint.
        while final_target_idx > 0
            && path_data.current_progress <= path_data.waypoints[final_target_idx - 1].progress
        {
            final_target_idx -= 1;
        }

        // Draw line from robot's current position to the target waypoint, then along the path to the final waypoint.
        if final_target_idx < path_data.waypoints.len() {
            let mut points_to_draw = vec![start_pos];
            points_to_draw.extend(
                path_data.waypoints[final_target_idx..]
                    .iter()
                    .map(|wp| wp.position),
            );

            if points_to_draw.len() > 1 {
                gizmos.linestrip(points_to_draw, PLANNED_PATH_COLOR);

                if let Some(final_wp) = path_data.waypoints.last() {
                    draw_path_endpoint(&mut gizmos, final_wp.position, PATH_ENDPOINT_COLOR);
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
                                DEPENDENCY_Z_OFFSET,
                            ));
                            break;
                        }
                    }

                    // Draw dependency line connecting the waiting point to the clearance point
                    if let Some(end_pos) = clearance_pos {
                        let start_pos =
                            Vec3::new(wp.position.x, wp.position.y, DEPENDENCY_Z_OFFSET);
                        let wait_wp = if i > 0 {
                            &path_data.waypoints[i - 1]
                        } else {
                            wp
                        };
                        let waiting_pos =
                            Vec3::new(wait_wp.position.x, wait_wp.position.y, DEPENDENCY_Z_OFFSET);
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
    let new_pos = Isometry3d::new(pos + Vec3::Z * PATH_ENDPOINT_Z_OFFSET, Quat::IDENTITY);
    gizmos.circle(new_pos, PATH_POINT_OUTER_RADIUS, color);
    gizmos.circle(new_pos, PATH_POINT_INNER_RADIUS, color);
}

fn draw_path_endpoint(gizmos: &mut Gizmos, pos: Vec3, color: Color) {
    let new_pos = pos + Vec3::Z * PATH_ENDPOINT_Z_OFFSET;
    let isometry_pos = Isometry3d::new(new_pos, Quat::IDENTITY);
    gizmos.circle(isometry_pos, PATH_POINT_OUTER_RADIUS, color);
    gizmos.circle(isometry_pos, PATH_POINT_INNER_RADIUS, color);
    gizmos.line(
        new_pos + Vec3::X * PATH_ENDPOINT_CROSS_RADIUS,
        new_pos - Vec3::X * PATH_ENDPOINT_CROSS_RADIUS,
        color,
    );
    gizmos.line(
        new_pos + Vec3::Y * PATH_ENDPOINT_CROSS_RADIUS,
        new_pos - Vec3::Y * PATH_ENDPOINT_CROSS_RADIUS,
        color,
    );
}
