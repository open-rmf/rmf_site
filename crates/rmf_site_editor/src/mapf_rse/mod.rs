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
        LocationTags, ModelMarker, NameInSite, Point, Pose, Robot,
    },
    CurrentWorkspace,
};
use mapf::negotiation::*;
use rmf_site_format::{NameOfSite, Original};

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
                    handle_start_negotiation,
                    handle_completed_negotiation,
                    // Ensures removal of MAPFDebugInfo takes effect before visualizing solution
                    visualise_selected_node.after(handle_start_negotiation),
                ),
            )
            .add_systems(
                Last,
                (
                    handle_changed_debug_goal,
                    handle_changed_collision,
                    handle_removed_plan_info,
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
        Self { time: 0.0 }
    }
}

fn get_occupancy_hashmap_from_grid(grid: &Grid) -> HashMap<i64, Vec<i64>> {
    let mut occupancy = HashMap::<i64, Vec<i64>>::new();
    for cell in grid.occupied.iter() {
        occupancy.entry(cell.y).or_default().push(cell.x);
    }
    for (_, column) in &mut occupancy {
        column.sort_unstable();
    }

    occupancy
}

fn handle_start_negotiation(
    locations: Query<&Point<Entity>, With<LocationTags>>,
    anchors: Query<&GlobalTransform>,
    mut negotiation_request: EventReader<NegotiationRequest>,
    negotiation_params: Res<NegotiationParams>,
    current_level: Res<CurrentLevel>,
    grids: Query<(Entity, &Grid)>,
    child_of: Query<&ChildOf>,
    robots: Query<
        (
            Entity,
            &Pose,
            &Affiliation<Entity>,
            Option<&Original<Pose>>,
            &DebugGoal,
        ),
        With<Robot>,
    >,
    robot_descriptions: Query<(&DifferentialDrive, &CircleCollision)>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
    mapf_info: Query<&mut MAPFDebugInfo>,
    mut commands: Commands,
    mut calculate_grid: EventWriter<CalculateGridRequest>,
) {
    let Some(site) = current_workspace.to_site(&open_sites) else {
        return;
    };

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

    negotiation_request.clear();

    let Some(grid) = grid else {
        warn!("No occupancy grid, sending calculate grid request");
        calculate_grid.write(CalculateGridRequest);
        return;
    };

    if let Some(plan_info) = mapf_info.get(site).ok() {
        if matches!(*plan_info, MAPFDebugInfo::InProgress { .. }) {
            warn!("Negotiation requested while another negotiation is in progress");
            return;
        }
        commands.entity(site).remove::<MAPFDebugInfo>();
    };

    let cell_size = grid.cell_size;
    let occupancy = get_occupancy_hashmap_from_grid(grid);

    // Agent
    let mut agents = BTreeMap::<String, Agent>::new();
    for (robot_entity, robot_pose, robot_group, robot_opose, debug_goal) in robots.iter() {
        let Some(location_entity) = debug_goal.entity else {
            continue;
        };
        let Some(Point(anchor_entity)) = locations.get(location_entity).ok() else {
            error!("Unable to query for location entity");
            continue;
        };

        let Ok(goal_transform) = anchors.get(*anchor_entity) else {
            warn!("Unable to query for robot's goal transform");
            continue;
        };
        let Some((differential_drive, circle_collision)) =
            robot_group.0.and_then(|e| robot_descriptions.get(e).ok())
        else {
            warn!("Unable to query for robot's collision model");
            continue;
        };
        let pose = if let Some(opose) = robot_opose {
            opose.0
        } else {
            *robot_pose
        };
        let goal_pos = goal_transform.translation();

        let to_discrete_xy = |x: f32, y: f32, cell_size: f32| -> [i64; 2] {
            Cell::from_point(Vec2::new(x, y), cell_size).to_xy()
        };

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
    }

    if agents.is_empty() {
        warn!("No agents with valid debug goal component");
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
    let new_plan_info = MAPFDebugInfo::InProgress {
        start_time: Instant::now(),
        task: TaskPool::new()
            .spawn_local(async move { negotiate(&scenario, Some(queue_length_limit)) }),
    };

    commands.entity(site).insert(new_plan_info);
}

fn handle_completed_negotiation(
    mut commands: Commands,
    mut debug_data: ResMut<NegotiationDebugData>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
    mut mapf_info: Query<&mut MAPFDebugInfo>,
    robots: Query<(Entity, &Pose, Option<&Original<Pose>>), With<Robot>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    robot_debug_materials: Query<&DebugMaterial, With<Robot>>,
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
                let mut longest_plan_duration_s = 0.0;

                for proposal in solution.proposals.iter() {
                    if let Some(last_waypt) = proposal.1.meta.trajectory.last() {
                        let plan_duration = last_waypt.time.duration_from_zero().as_secs_f32();
                        if plan_duration > longest_plan_duration_s {
                            longest_plan_duration_s = plan_duration;
                        }
                    }
                }

                let mut entity_id_map = HashMap::new();

                for (k, robot_entity_str) in name_map.iter() {
                    let robot_entity = bits_string_to_entity(robot_entity_str);

                    // Inserts original poses for each robot if no original pose
                    if let Some((_, pose, opose)) = robots.get(robot_entity).ok() {
                        if opose.is_none() {
                            commands
                                .entity(robot_entity)
                                .insert(Original::<Pose>(*pose));
                        }
                    }

                    // Inserts DebugMaterial component for each robot if don't exist
                    if robot_debug_materials.get(robot_entity).ok().is_none() {
                        commands.entity(robot_entity).insert(DebugMaterial {
                            handle: materials.add(StandardMaterial {
                                base_color: Color::srgb_from_array(ColorPicker::get_color()),
                                unlit: true,
                                ..Default::default()
                            }),
                        });
                    }

                    entity_id_map.insert(*k, robot_entity);
                }

                debug_data.reset();
                commands.entity(site).insert(MAPFDebugInfo::Success {
                    longest_plan_duration_s,
                    elapsed_time,
                    solution,
                    entity_id_map,
                    negotiation_history,
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

fn handle_changed_debug_goal(
    debug_goals_changed: Query<(Entity, Option<&Original<Pose>>, &DebugGoal), Changed<DebugGoal>>,
    debug_goals_added: Query<(), Added<DebugGoal>>,
    mut negotiation_request: EventWriter<NegotiationRequest>,
    mut change_pose: EventWriter<Change<Pose>>,
    mut commands: Commands,
) {
    let mut any_debug_goal_changed = false;
    for (entity, original_pose, debug_goal) in debug_goals_changed.iter() {
        if debug_goal.location.is_empty() {
            if let Some(pose) = original_pose {
                change_pose.write(Change::new(pose.0, entity));
                commands.entity(entity).remove::<Original<Pose>>();
            }
        }

        // If it is not a newly-added component
        if !debug_goals_added.get(entity).ok().is_some() {
            any_debug_goal_changed = true;
        }
    }
    if any_debug_goal_changed {
        negotiation_request.write(NegotiationRequest);
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

fn handle_removed_plan_info(
    path_visuals: Query<Entity, With<PathVisualMarker>>,
    mut removed_plan_info: RemovedComponents<MAPFDebugInfo>,
    mut commands: Commands,
) {
    if !removed_plan_info.is_empty() {
        for e in path_visuals.iter() {
            commands.entity(e).despawn();
        }
        removed_plan_info.clear();
    }
}
