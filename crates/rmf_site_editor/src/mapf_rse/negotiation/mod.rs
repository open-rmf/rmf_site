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
    collections::{BTreeMap, HashMap, VecDeque},
    fmt::Debug,
    time::{Duration, Instant},
};

use crate::{
    color_picker::ColorPicker,
    occupancy::{calculate_grid, Cell, Grid},
    site::{
        Affiliation, CircleCollision, CurrentLevel, DifferentialDrive, GoToPlace, Group,
        LocationTags, ModelMarker, NameInSite, Point, Pose, Robot, Task as RobotTask,
    },
    CurrentWorkspace,
};
use mapf::negotiation::*;
use rmf_site_format::NameOfSite;

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
            .init_resource::<NegotiationTask>()
            .add_plugins(NegotiationDebugPlugin::default())
            .add_systems(
                Update,
                (
                    start_compute_negotiation.before(calculate_grid),
                    handle_compute_negotiation_complete,
                    visualise_selected_node,
                ),
            );
    }
}

#[derive(Event)]
pub struct NegotiationRequest;

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

#[derive(Debug, Default, Clone)]
pub enum NegotiationTaskStatus {
    #[default]
    NotStarted,
    InProgress {
        start_time: Instant,
    },
    Complete {
        longest_plan_duration: f32,
        colors: Vec<[f32; 3]>,
        elapsed_time: Duration,
        solution: Option<NegotiationNode>,
        negotiation_history: Vec<NegotiationNode>,
        entity_id_map: HashMap<usize, Entity>,
        error_message: Option<String>,
        conflicting_endpoints: Vec<(Entity, Entity)>,
    },
}

impl NegotiationTaskStatus {
    pub fn is_in_progress(&self) -> bool {
        matches!(self, NegotiationTaskStatus::InProgress { .. })
    }
}

#[derive(Debug, Resource)]
pub struct NegotiationTask {
    task: Task<
        Result<
            (
                NegotiationNode,
                Vec<NegotiationNode>,
                HashMap<usize, String>,
            ),
            NegotiationError,
        >,
    >,
    pub status: NegotiationTaskStatus,
}

impl Default for NegotiationTask {
    fn default() -> Self {
        Self {
            task: TaskPool::new().spawn_local(async move {
                Err(NegotiationError::PlanningImpossible(
                    "Not started yet".into(),
                ))
            }),
            status: NegotiationTaskStatus::NotStarted,
        }
    }
}

#[derive(Resource)]
pub struct NegotiationDebugData {
    pub show_debug_panel: bool,
    pub selected_negotiation_node: Option<usize>,
    pub playback_speed: f32,
    pub time: f32,
}

impl NegotiationDebugData {
    fn reset(&mut self) {
        self.time = 0.0;
        self.selected_negotiation_node = None;
    }
}

impl Default for NegotiationDebugData {
    fn default() -> Self {
        Self {
            show_debug_panel: false,
            selected_negotiation_node: None,
            playback_speed: 1.0,
            time: 0.0,
        }
    }
}

pub fn handle_compute_negotiation_complete(
    mut commands: Commands,
    mut negotiation_debug_data: ResMut<NegotiationDebugData>,
    mut negotiation_task: ResMut<NegotiationTask>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
) {
    fn bits_string_to_entity(bits_string: &str) -> Entity {
        // SAFETY: This assumes function input bits_string to be output from entity.to_bits().to_string()
        // Currently, this is fetched from start_compute_negotiation fn, e.g. the key of BTreeMap in scenario.agents
        let bits = u64::from_str_radix(bits_string, 10).expect("Invalid entity id");
        Entity::from_bits(bits)
    }

    let NegotiationTaskStatus::InProgress { start_time } = negotiation_task.status else {
        return;
    };

    if let Some(result) = check_ready(&mut negotiation_task.task) {
        let elapsed_time = start_time.elapsed();
        let mut colors = Vec::new();
        let mut longest_plan_duration = 0.0;

        let Some(site) = current_workspace.to_site(&open_sites) else {
            error!("Cannot find current site");
            return;
        };

        match result {
            Ok((solution, negotiation_history, name_map)) => {
                negotiation_debug_data.selected_negotiation_node = Some(solution.id);
                for proposal in solution.proposals.iter() {
                    if let Some(last_waypt) = proposal.1.meta.trajectory.last() {
                        let plan_duration = last_waypt.time.duration_from_zero().as_secs_f32();
                        if plan_duration > longest_plan_duration {
                            longest_plan_duration = plan_duration;
                        }
                    }
                    colors.push(ColorPicker::get_color());
                }

                commands.entity(site).insert(MAPFDebugInfo::Success {
                    longest_plan_duration_s: longest_plan_duration,
                    colors: colors.clone(),
                    elapsed_time: elapsed_time,
                    solution: solution.clone(),
                    entity_id_map: name_map
                        .clone()
                        .into_iter()
                        .map(|(id, bits_string)| (id, bits_string_to_entity(&bits_string)))
                        .collect(),
                    path_mesh_info: VecDeque::new(),
                    negotiation_history: negotiation_history.clone(),
                });

                negotiation_task.status = NegotiationTaskStatus::Complete {
                    longest_plan_duration,
                    colors,
                    elapsed_time,
                    solution: Some(solution),
                    negotiation_history,
                    entity_id_map: name_map
                        .into_iter()
                        .map(|(id, bits_string)| (id, bits_string_to_entity(&bits_string)))
                        .collect(),
                    error_message: None,
                    conflicting_endpoints: Vec::new(),
                };
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

                negotiation_task.status = NegotiationTaskStatus::Complete {
                    longest_plan_duration,
                    colors,
                    elapsed_time: elapsed_time,
                    solution: None,
                    negotiation_history: negotiation_history,
                    entity_id_map: entity_id_map,
                    error_message: err_msg,
                    conflicting_endpoints: conflicts,
                };
            }
        };
    }
}

pub fn start_compute_negotiation(
    locations: Query<(&NameInSite, &Point<Entity>), With<LocationTags>>,
    anchors: Query<&GlobalTransform>,
    negotiation_request: EventReader<NegotiationRequest>,
    negotiation_params: Res<NegotiationParams>,
    mut negotiation_debug_data: ResMut<NegotiationDebugData>,
    current_level: Res<CurrentLevel>,
    grids: Query<(Entity, &Grid)>,
    child_of: Query<&ChildOf>,
    robots: Query<(Entity, &NameInSite, &Pose, &Affiliation<Entity>), With<Robot>>,
    robot_descriptions: Query<(&DifferentialDrive, &CircleCollision)>,
    tasks: Query<(&RobotTask, &GoToPlace)>,
    mut negotiation_task: ResMut<NegotiationTask>,
) {
    if negotiation_request.len() == 0 {
        return;
    }

    if negotiation_task.status.is_in_progress() {
        warn!("Negotiation requested while another negotiation is in progress");
        return;
    }

    negotiation_debug_data.selected_negotiation_node = None;

    // Occupancy
    let mut occupancy = HashMap::<i64, Vec<i64>>::new();
    let mut cell_size = 1.0;
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
    match grid {
        Some(grid) => {
            cell_size = grid.cell_size;
            for cell in grid.occupied.iter() {
                occupancy.entry(cell.y).or_default().push(cell.x);
            }
            for (_, column) in &mut occupancy {
                column.sort_unstable();
            }
        }
        None => {
            occupancy.entry(0).or_default().push(0);
            warn!("No occupancy grid found, defaulting to empty");
        }
    }

    // Agent
    let mut agents = BTreeMap::<String, Agent>::new();
    // Only loop tasks that have specified a valid robot
    for (task, go_to_place) in tasks.iter() {
        // Identify robot
        let robot_name = task.robot();
        for (robot_entity, robot_site_name, robot_pose, robot_group) in robots.iter() {
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
                        let goal_pos = goal_transform.translation();
                        let agent = Agent {
                            start: to_discrete_xy(
                                robot_pose.trans[0],
                                robot_pose.trans[1],
                                cell_size,
                            ),
                            yaw: f64::from(robot_pose.rot.yaw().radians()),
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
    let start_time = Instant::now();
    negotiation_task.status = NegotiationTaskStatus::InProgress { start_time };
    negotiation_task.task =
        TaskPool::new().spawn_local(async move { negotiate(&scenario, Some(queue_length_limit)) });
}

fn to_discrete_xy(x: f32, y: f32, cell_size: f32) -> [i64; 2] {
    Cell::from_point(Vec2::new(x, y), cell_size).to_xy()
}
