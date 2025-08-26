/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use bevy::{
    ecs::hierarchy::ChildOf,
    prelude::*,
    tasks::{futures::check_ready, Task, TaskPool},
};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    time::{Duration, Instant},
};

use crate::{
    color_picker::ColorPicker,
    occupancy::{CalculateGridRequest, Cell, Grid},
    site::{
        Affiliation, Change, CircleCollision, CurrentLevel, DifferentialDrive, GoToPlace, Group,
        LocationTags, ModelMarker, NameInSite, Point, Pose, Robot, Task as RobotTask,
    },
    CurrentWorkspace,
};
use mapf::negotiation::*;
use rmf_site_format::{NameOfSite, Original, TaskKind};

use mapf::negotiation::{Agent, Obstacle, Scenario as MapfScenario};

pub mod debug_panel;
pub use debug_panel::*;

pub mod visual;
pub use visual::*;

#[derive(Default)]
pub struct NegotiationPlugin;

impl Plugin for NegotiationPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<NegotiationRequest>()
            .init_resource::<NegotiationParams>()
            .init_resource::<NegotiationDebugData>()
            .init_resource::<DebuggerSettings>()
            .add_plugins(NegotiationDebugPlugin::default())
            .add_systems(
                Update,
                (
                    start_compute_negotiation,
                    handle_changed_collision,
                    handle_removed_tasks,
                    handle_changed_tasks,
                    handle_compute_negotiation_complete,
                    visualise_selected_node,
                    remove_robot_path_entities,
                ),
            );
    }
}

#[derive(Event)]
pub struct NegotiationRequest;

// Algorithm-specific parameters
#[derive(Debug, Clone, Resource)]
pub struct NegotiationParams {
    pub queue_length_limit: usize,
}

impl Default for NegotiationParams {
    fn default() -> Self {
        Self {
            queue_length_limit: 1_000_000,
        }
    }
}

#[derive(Resource)]
pub struct NegotiationDebugData {
    pub time: f32,
    pub colors: Vec<[f32; 3]>,
}

impl NegotiationDebugData {
    fn reset(&mut self) {
        self.time = 0.0;
    }
}

#[derive(Resource)]
pub struct DebuggerSettings {
    pub playback_speed: f32,
}

impl Default for DebuggerSettings {
    fn default() -> Self {
        Self {
            playback_speed: 1.0,
        }
    }
}

impl Default for NegotiationDebugData {
    fn default() -> Self {
        Self {
            time: 0.0,
            colors: Vec::new(),
        }
    }
}

pub fn remove_robot_path_entities(
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
    mapf_info: Query<&MAPFDebugInfo>,
    path_visuals: Query<Entity, With<PathVisualMarker>>,
    mut commands: Commands,
) {
    let Some(site) = current_workspace.to_site(&open_sites) else {
        return;
    };

    if mapf_info.get(site).ok().is_none() {
        for e in path_visuals.iter() {
            commands.entity(e).despawn();
        }
    };
}

pub fn handle_compute_negotiation_complete(
    mut commands: Commands,
    mut debug_data: ResMut<NegotiationDebugData>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
    mut mapf_info: Query<&mut MAPFDebugInfo>,
    robots: Query<(Entity, &Pose, Option<&Original<Pose>>), With<Robot>>,
) {
    fn bits_string_to_entity(bits_string: &str) -> Entity {
        // SAFETY: This assumes function input bits_string to be output from entity.to_bits().to_string()
        // Currently, this is fetched from start_compute_negotiation fn, e.g. the key of BTreeMap in scenario.agents
        let bits = u64::from_str_radix(bits_string, 10).expect("Invalid entity id");
        Entity::from_bits(bits)
    }

    let Some(site) = current_workspace.to_site(&open_sites) else {
        return;
    };

    let Some(mut plan_info) = mapf_info.get_mut(site).ok() else {
        return;
    };

    let MAPFDebugInfo::InProgress {
        start_time,
        ref mut task,
    } = *plan_info
    else {
        return;
    };

    if let Some(result) = check_ready(task) {
        let elapsed_time = start_time.elapsed();

        let Some(site) = current_workspace.to_site(&open_sites) else {
            return;
        };

        match result {
            Ok((solution, negotiation_history, name_map)) => {
                let colors = &mut debug_data.colors;
                let mut longest_plan_duration = 0.0;

                if colors.len() < solution.proposals.len() {
                    let num_colors_required = solution.proposals.len() - colors.len();
                    for _ in 0..num_colors_required {
                        colors.push(ColorPicker::get_color());
                    }
                }

                for proposal in solution.proposals.iter() {
                    if let Some(last_waypt) = proposal.1.meta.trajectory.last() {
                        let plan_duration = last_waypt.time.duration_from_zero().as_secs_f32();
                        if plan_duration > longest_plan_duration {
                            longest_plan_duration = plan_duration;
                        }
                    }
                }

                // Inserts original poses of robot
                for (_, robot_entity_str) in name_map.iter() {
                    let robot_entity = bits_string_to_entity(robot_entity_str);
                    if let Some((_, pose, _)) = robots.get(robot_entity).ok() {
                        commands
                            .entity(robot_entity)
                            .insert(Original::<Pose>(*pose));
                    }
                }

                debug_data.reset();
                commands.entity(site).insert(MAPFDebugInfo::Success {
                    longest_plan_duration_s: longest_plan_duration,
                    elapsed_time: elapsed_time,
                    solution: solution.clone(),
                    entity_id_map: name_map
                        .clone()
                        .into_iter()
                        .map(|(id, bits_string)| (id, bits_string_to_entity(&bits_string)))
                        .collect(),
                    negotiation_history: negotiation_history.clone(),
                });
            }
            Err(err) => {
                let mut negotiation_history = Vec::new();
                let mut entity_id_map = HashMap::new();
                let mut err_msg = Some(err.to_string());
                let mut conflicts = Vec::new();

                match err {
                    NegotiationError::PlanningImpossible(msg) => {
                        if let Some(err_str) = err_msg {
                            err_msg = Some([err_str, msg].join(" "));
                        }
                    }
                    NegotiationError::ConflictingEndpoints(conflicts_map) => {
                        conflicts = conflicts_map
                            .into_iter()
                            .map(|(a, b)| (bits_string_to_entity(&a), bits_string_to_entity(&b)))
                            .collect();
                    }
                    NegotiationError::PlanningFailed((neg_history, name_map)) => {
                        negotiation_history = neg_history;
                        entity_id_map = name_map
                            .into_iter()
                            .map(|(id, bits_string)| (id, bits_string_to_entity(&bits_string)))
                            .collect();
                    }
                }

                commands.entity(site).insert(MAPFDebugInfo::Failed {
                    error_message: err_msg.clone(),
                    conflicts: conflicts.clone(),
                    negotiation_history: negotiation_history.clone(),
                    entity_id_map: entity_id_map.clone(),
                });
            }
        };
    }
}

fn handle_removed_tasks(
    mut removed_tasks: RemovedComponents<RobotTask>,
    mut negotiation_request: EventWriter<NegotiationRequest>,
) {
    if !removed_tasks.is_empty() {
        negotiation_request.write(NegotiationRequest);
        removed_tasks.clear();
    }
}

fn handle_changed_tasks(
    robot_tasks_changed: Query<&RobotTask, Changed<RobotTask>>,
    mut negotiation_request: EventWriter<NegotiationRequest>,
) {
    for robot_task in robot_tasks_changed.iter() {
        if let RobotTask::Direct(robot_task_request) = robot_task {
            if robot_task_request.request.is_valid()
                && robot_task_request.request.category() == GoToPlace::label()
            {
                negotiation_request.write(NegotiationRequest);
                return;
            }
        }
    }
}

fn handle_changed_collision(
    collision_changed: Query<Entity, Changed<CircleCollision>>,
    mut negotiation_request: EventWriter<NegotiationRequest>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
    mapf_info: Query<&MAPFDebugInfo>,
) {
    if !collision_changed.is_empty() {
        let Some(site) = current_workspace.to_site(&open_sites) else {
            return;
        };

        if mapf_info.get(site).ok().is_some() {
            negotiation_request.write(NegotiationRequest);
        }
    }
}

pub fn is_planning_in_progress(
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
    mapf_info: Query<&MAPFDebugInfo>,
) -> bool {
    let Some(site) = current_workspace.to_site(&open_sites) else {
        return false;
    };

    let Some(plan_info) = mapf_info.get(site).ok() else {
        return false;
    };

    if matches!(*plan_info, MAPFDebugInfo::InProgress { .. }) {
        warn!("Negotiation requested while another negotiation is in progress");
        return true;
    }
    return false;
}

pub fn start_compute_negotiation(
    locations: Query<(&NameInSite, &Point<Entity>), With<LocationTags>>,
    anchors: Query<&GlobalTransform>,
    mut negotiation_request: EventReader<NegotiationRequest>,
    negotiation_params: Res<NegotiationParams>,
    current_level: Res<CurrentLevel>,
    grids: Query<(Entity, &Grid)>,
    child_of: Query<&ChildOf>,
    mut robots: Query<
        (
            Entity,
            &NameInSite,
            &Pose,
            &Affiliation<Entity>,
            Option<&Original<Pose>>,
        ),
        With<Robot>,
    >,
    robot_descriptions: Query<(&DifferentialDrive, &CircleCollision)>,
    tasks: Query<(&RobotTask, &GoToPlace)>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
    mapf_info: Query<&MAPFDebugInfo>,
    mut commands: Commands,
    mut change_pose: EventWriter<Change<Pose>>,
    mut calculate_grid: EventWriter<CalculateGridRequest>,
) {
    if negotiation_request.is_empty() {
        return;
    }

    let grid = grids.iter().find_map(|(grid_entity, grid)| {
        if let Some(level_entity) = current_level.0 {
            if child_of
                .get(grid_entity)
                .is_ok_and(|co| co.parent() == level_entity)
            {
                Some(grid)
            } else {
                None
            }
        } else {
            None
        }
    });

    let Some(grid) = grid else {
        warn!("No occupancy grid, sending calculate grid request");
        calculate_grid.write(CalculateGridRequest);
        return;
    };

    negotiation_request.clear();

    let Some(site) = current_workspace.to_site(&open_sites) else {
        return;
    };

    if is_planning_in_progress(open_sites, current_workspace, mapf_info) {
        warn!("Planning is in progress!");
        return;
    }

    commands.entity(site).remove::<MAPFDebugInfo>();
    // Reset back to start
    for (robot_entity, _, _, _, robot_opose) in robots.iter_mut() {
        if let Some(opose) = robot_opose {
            change_pose.write(Change::new(opose.0, robot_entity));
            commands.entity(robot_entity).remove::<Original<Pose>>();
        }
    }

    let mut occupancy = HashMap::<i64, Vec<i64>>::new();
    let cell_size = grid.cell_size;
    for cell in grid.occupied.iter() {
        occupancy.entry(cell.y).or_default().push(cell.x);
    }
    for (_, column) in &mut occupancy {
        column.sort_unstable();
    }

    // Agent
    let mut agents = BTreeMap::<String, Agent>::new();
    // Only loop tasks that have specified a valid robot
    for (task, go_to_place) in tasks.iter() {
        // Identify robot
        let robot_name = task.robot();
        for (robot_entity, robot_site_name, robot_pose, robot_group, robot_opose) in robots.iter() {
            if robot_name == robot_site_name.0 {
                // Match location to entity
                for (location_name, Point(anchor_entity)) in locations.iter() {
                    if location_name.0 == go_to_place.location {
                        let Ok(goal_transform) = anchors.get(*anchor_entity) else {
                            warn!("Unable to get robot's goal transform");
                            continue;
                        };
                        let Some((differential_drive, circle_collision)) =
                            robot_group.0.and_then(|e| robot_descriptions.get(e).ok())
                        else {
                            warn!("Unable to get robot's collision model");
                            continue;
                        };
                        let pose = if let Some(opose) = robot_opose {
                            opose.0
                        } else {
                            *robot_pose
                        };
                        let goal_pos = goal_transform.translation();
                        let agent = Agent {
                            start: to_discrete_xy(pose.trans[0], pose.trans[1], cell_size),
                            yaw: f64::from(pose.rot.yaw().radians()),
                            goal: to_discrete_xy(goal_pos.x, goal_pos.y, cell_size),
                            radius: f64::from(circle_collision.radius),
                            speed: f64::from(differential_drive.translational_speed),
                            spin: f64::from(differential_drive.rotational_speed),
                        };
                        let agent_id = robot_entity.to_bits().to_string();
                        agents.insert(agent_id, agent);
                        break;
                    }
                }
                break;
            }
        }
    }

    if agents.len() == 0 {
        warn!("No agents with valid GoToPlace task");
        return;
    }

    info!(
        "Successfully sent planning request for {} agents!",
        agents.len()
    );

    let scenario = MapfScenario {
        agents: agents,
        obstacles: Vec::<Obstacle>::new(),
        occupancy: occupancy,
        cell_size: f64::from(cell_size),
        camera_bounds: None,
    };
    let queue_length_limit = negotiation_params.queue_length_limit;

    // Execute asynchronously
    commands.entity(site).insert(MAPFDebugInfo::InProgress {
        start_time: Instant::now(),
        task: TaskPool::new()
            .spawn_local(async move { negotiate(&scenario, Some(queue_length_limit)) }),
    });
}

fn to_discrete_xy(x: f32, y: f32, cell_size: f32) -> [i64; 2] {
    Cell::from_point(Vec2::new(x, y), cell_size).to_xy()
}
