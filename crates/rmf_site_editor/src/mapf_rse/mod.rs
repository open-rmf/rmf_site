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

use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use std::{
    collections::{BTreeMap, HashMap},
    fmt::Debug,
    time::{Duration, Instant},
};

use crate::{
    color_picker::ColorPicker,
    layers::ZLayer,
    occupancy::{CalculateGridRequest, Cell, Grid, OccupancyVisualMarker},
    site::{
        line_stroke_transform, Affiliation, Change, CircleCollision, CurrentLevel,
        DifferentialDrive, GoToPlace, Group, LocationTags, ModelMarker, Point, Pose, Robot,
        SiteAssets,
    },
    CurrentWorkspace,
};
use mapf::negotiation::*;
use rmf_site_format::{NameOfSite, Original};

use mapf::motion::{se2::WaypointSE2, TimeCmp};
use mapf::negotiation::{Agent, Obstacle, Scenario as MapfScenario};

pub mod debug_panel;
pub use debug_panel::*;

pub mod visual;
use std::thread;
pub use visual::*;

#[derive(Default)]
pub struct NegotiationPlugin;

impl Plugin for NegotiationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NegotiationParams>()
            .init_resource::<NegotiationDebugData>()
            .init_resource::<DebuggerSettings>()
            .init_resource::<PlanningProgressChannel>()
            .add_plugins(NegotiationDebugPlugin::default())
            .add_systems(
                Update,
                (
                    handle_start_negotiation.before(visualise_selected_node),
                    handle_completed_negotiation,
                    visualise_selected_node.before(handle_debug_panel_changed),
                ),
            )
            .add_systems(
                Last,
                (
                    handle_changed_debug_goal,
                    handle_changed_collision,
                    handle_changed_plan_info,
                    handle_removed_plan_info,
                ),
            );
    }
}

#[derive(Event)]
pub struct NegotiationRequest;

use crossbeam_channel::{unbounded, Receiver, Sender};

pub struct PlanningCompleted {
    result: Result<
        (
            NegotiationNode,
            Vec<NegotiationNode>,
            HashMap<usize, String>,
        ),
        NegotiationError,
    >,
}
/// Channels that give incremental updates about what models have been fetched.
#[derive(Debug, Resource)]
pub struct PlanningProgressChannel {
    pub sender: Sender<PlanningCompleted>,
    pub receiver: Receiver<PlanningCompleted>,
}

impl Default for PlanningProgressChannel {
    fn default() -> Self {
        let (sender, receiver) = unbounded();
        Self { sender, receiver }
    }
}

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
    pub start_pointers: Vec<usize>,
    pub trajectory_lengths: Vec<usize>,
    pub circle_entities: Vec<Entity>,
    pub rectangle_entities: Vec<Entity>,
}

impl NegotiationDebugData {
    fn reset(&mut self) {
        self.time = 0.0;
        self.trajectory_lengths.clear();
        self.start_pointers.clear();
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
            start_pointers: Vec::new(),
            trajectory_lengths: Vec::new(),
            circle_entities: Vec::new(),
            rectangle_entities: Vec::new(),
        }
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

fn bits_string_to_entity(bits_string: &str) -> Entity {
    // SAFETY: This assumes function input bits_string to be output from entity.to_bits().to_string()
    // Currently, this is fetched from start_compute_negotiation fn, e.g. the key of BTreeMap in scenario.agents
    let bits = u64::from_str_radix(bits_string, 10).expect("Invalid entity id");
    Entity::from_bits(bits)
}

fn name_map_to_entity_map(name_map: &HashMap<usize, String>) -> HashMap<usize, Entity> {
    let mut entity_id_map = HashMap::new();
    for (k, robot_entity_str) in name_map.iter() {
        let robot_entity = bits_string_to_entity(robot_entity_str);
        entity_id_map.insert(*k, robot_entity);
    }

    entity_id_map
}

fn conflicts_map_to_vec(conflicts_map: HashMap<String, String>) -> Vec<(Entity, Entity)> {
    conflicts_map
        .into_iter()
        .map(|(a, b)| (bits_string_to_entity(&a), bits_string_to_entity(&b)))
        .collect()
}

fn get_robot_z_from_time(t: f32, longest_plan_duration_s: f32) -> f32 {
    let mut z = ZLayer::RobotPath.to_z();
    if let Some(next_z_layer) = ZLayer::RobotPath.next() {
        z += (1.0 - (t / longest_plan_duration_s))
            * ZLayer::get_z_offset(ZLayer::RobotPath, next_z_layer);
    } else {
        error!("No Z-layer after robot path!");
    }
    z
}

fn get_start_pointers_from_trajectory_lengths(trajectory_lengths: &Vec<usize>) -> Vec<usize> {
    let mut start_pointers = Vec::new();
    start_pointers.push(0);
    if trajectory_lengths.len() <= 1 {
        return start_pointers;
    }
    let mut num_total_waypoint_pairs = 0;
    for i in 0..trajectory_lengths.len() - 1 {
        // One trajectory contains N mesh entities
        // where N is trajectory length - 1, e.g. number of waypoint pairs in a trajectory
        let num_waypoint_pairs = trajectory_lengths[i] - 1;
        num_total_waypoint_pairs += num_waypoint_pairs;
        start_pointers.push(num_total_waypoint_pairs);
    }
    start_pointers
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
    mut debug_data: ResMut<NegotiationDebugData>,
    planning_progress_channel: Res<PlanningProgressChannel>,
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
    debug_data.reset();

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

        info!("Removing MAPF Debug Info");
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

    commands.entity(site).insert(MAPFDebugInfo::InProgress {
        start_time: Instant::now(),
    });

    let tx = planning_progress_channel.sender.clone();
    thread::spawn(move || {
        if let Err(err) = tx.send(PlanningCompleted {
            result: negotiate(&scenario, Some(queue_length_limit)),
        }) {
            error!("Failed sending planning completed {:?}", err);
        }
    });
}

fn handle_completed_negotiation(
    mut commands: Commands,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
    mapf_info: Query<&MAPFDebugInfo>,
    planning_progress_channel: Res<PlanningProgressChannel>,
) {
    let Some(site) = current_workspace.to_site(&open_sites) else {
        return;
    };

    let Some(plan_info) = mapf_info.get(site).ok() else {
        return;
    };

    let Some((start_time, result)) = (if let MAPFDebugInfo::InProgress { start_time } = plan_info {
        if let Ok(planning_results) = planning_progress_channel.receiver.try_recv() {
            Some((start_time, planning_results.result))
        } else {
            None
        }
    } else {
        None
    }) else {
        return;
    };

    let elapsed_time = start_time.elapsed();

    let Some(site) = current_workspace.to_site(&open_sites) else {
        return;
    };

    match result {
        Ok((solution, negotiation_history, name_map)) => {
            let longest_plan_duration_s = if let Some(max_duration_s) = solution
                .proposals
                .iter()
                .map(|proposal| {
                    if let Some(last_waypt) = proposal.1.meta.trajectory.last() {
                        last_waypt.time.duration_from_zero().as_secs_f32()
                    } else {
                        error!("Trajectory is empty, setting trajectory duration to 0s!");
                        0.0
                    }
                })
                .reduce(f32::max)
            {
                max_duration_s
            } else {
                error!("Solution proposals are empty, setting longest plan duration to 0s!");
                0.0
            };

            commands.entity(site).insert(MAPFDebugInfo::Success {
                longest_plan_duration_s,
                elapsed_time,
                solution: solution.clone(),
                entity_id_map: name_map_to_entity_map(&name_map),
                negotiation_history: negotiation_history.clone(),
            });
        }
        Err(err) => {
            let mut negotiation_history = Vec::new();
            let mut entity_id_map = HashMap::new();
            let mut error_message = Some(err.to_string());
            let mut conflicts = Vec::new();

            match err {
                NegotiationError::PlanningImpossible(msg) => {
                    if let Some(err_str) = error_message {
                        error_message = Some([err_str, msg.to_string()].join(" "));
                    }
                }
                NegotiationError::ConflictingEndpoints(conflicts_map) => {
                    conflicts = conflicts_map_to_vec(conflicts_map.clone());
                }
                NegotiationError::PlanningFailed((neg_history, name_map)) => {
                    negotiation_history = neg_history.to_vec();
                    entity_id_map = name_map_to_entity_map(&name_map);
                }
            }

            commands.entity(site).insert(MAPFDebugInfo::Failed {
                error_message,
                conflicts,
                negotiation_history,
                entity_id_map,
            });
        }
    };
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

        // If it is not a newly-added component (changed debug goal) send replanning request
        // or its a newly-added component with valid location
        if !debug_goals_added.get(entity).ok().is_some() || !debug_goal.location.is_empty() {
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

fn handle_changed_plan_info(
    changed_plan_info: Query<&MAPFDebugInfo, Changed<MAPFDebugInfo>>,
    robots: Query<(Entity, &Affiliation<Entity>, &Pose, Option<&Original<Pose>>), With<Robot>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    robot_debug_materials: Query<&DebugMaterial, With<Robot>>,
    mut commands: Commands,
    mut change_pose: EventWriter<Change<Pose>>,
    mut debug_data: ResMut<NegotiationDebugData>,
    site_assets: Res<SiteAssets>,
    current_level: Res<CurrentLevel>,
    robot_materials: Query<&DebugMaterial, With<Robot>>,
    robot_descriptions: Query<&CircleCollision, (With<ModelMarker>, With<Group>)>,
    mut path_meshes: Query<
        (
            &mut Visibility,
            &mut MeshMaterial3d<StandardMaterial>,
            &mut Transform,
        ),
        With<PathVisualMarker>,
    >,
    display_mapf_debug: Res<MAPFDebugDisplay>,
) {
    let Some(level_entity) = current_level.0 else {
        return;
    };
    let Some(plan_info) = changed_plan_info.iter().next() else {
        return;
    };

    info!("Plan info changed");
    match plan_info {
        MAPFDebugInfo::Success {
            longest_plan_duration_s,
            elapsed_time: _,
            solution,
            entity_id_map,
            negotiation_history: _,
        } => {
            for proposal in solution.proposals.iter() {
                debug_data
                    .trajectory_lengths
                    .push(proposal.1.meta.trajectory.len());
            }

            for (mut visibility, _, _) in path_meshes.iter_mut() {
                *visibility = Visibility::Hidden;
            }

            debug_data.start_pointers =
                get_start_pointers_from_trajectory_lengths(&debug_data.trajectory_lengths);

            for (i, proposal) in solution.proposals.iter().enumerate() {
                let Some(robot_entity) = entity_id_map.get(&proposal.0) else {
                    error!("Unable to query for robot entity from entity id map");
                    continue;
                };
                let Some((_, affiliation, _, _)) = robots.get(*robot_entity).ok() else {
                    warn!("Unable to query for robot entity's affiliation");
                    continue;
                };
                let Some(description_entity) = affiliation.0 else {
                    warn!("Unable to query for robot's model description entity");
                    continue;
                };

                let Some(collision_model) = robot_descriptions.get(description_entity).ok() else {
                    error!("No circle collision model found for robot's model description");
                    continue;
                };

                let material = if let Some(material) = robot_materials.get(*robot_entity).ok() {
                    material.clone()
                } else {
                    let material = DebugMaterial {
                        handle: materials.add(StandardMaterial {
                            base_color: Color::srgb_from_array(ColorPicker::get_color()),
                            unlit: true,
                            ..Default::default()
                        }),
                    };

                    let material_copy = material.clone();
                    // Inserts DebugMaterial component for each robot if don't exist
                    if robot_debug_materials.get(*robot_entity).ok().is_none() {
                        commands.entity(*robot_entity).insert(material);
                    }
                    material_copy
                };

                let waypoint_to_xyz = |tf: TimeCmp<WaypointSE2>| {
                    let time = tf.time.as_secs_f32();
                    Vec3::new(
                        tf.position.translation.x as f32,
                        tf.position.translation.y as f32,
                        get_robot_z_from_time(time, *longest_plan_duration_s),
                    )
                };

                let start_ptr = debug_data.start_pointers[i];
                for (waypt_id, slice) in proposal.1.meta.trajectory.windows(2).enumerate() {
                    let start_pos = waypoint_to_xyz(slice[0]);
                    let end_pos = waypoint_to_xyz(slice[1]);

                    let radius = collision_model.radius;

                    let rectangle_tf = line_stroke_transform(&start_pos, &end_pos, radius * 2.0);
                    let start_circle_tf = Transform::from_translation(start_pos)
                        .with_scale([radius, radius, 1.0].into());
                    let end_circle_tf = Transform::from_translation(end_pos)
                        .with_scale([radius, radius, 1.0].into());

                    let id = start_ptr + waypt_id;

                    let visibility = if display_mapf_debug.show {
                        Visibility::Visible
                    } else {
                        Visibility::Hidden
                    };

                    if id < debug_data.rectangle_entities.len() {
                        // reuse existing mesh entities
                        let mut change_material_and_tf = |mesh_entity_id, tf_to| {
                            if let Some((mut vis, mut material_mut, mut tf)) =
                                path_meshes.get_mut(mesh_entity_id).ok()
                            {
                                *material_mut = MeshMaterial3d(material.handle.clone());
                                *tf = tf_to;
                                *vis = visibility;
                            }
                        };

                        let rect_entity_id = debug_data.rectangle_entities[id];
                        let start_circle_entity_id = debug_data.circle_entities[2 * id];
                        let end_circle_entity_id = debug_data.circle_entities[2 * id + 1];

                        change_material_and_tf(rect_entity_id, rectangle_tf);
                        change_material_and_tf(start_circle_entity_id, start_circle_tf);
                        change_material_and_tf(end_circle_entity_id, end_circle_tf);
                    } else {
                        // not enough mesh entities, have to insert new ones
                        let mut spawn_as_child_and_get_id = |entity| {
                            commands
                                .spawn(entity)
                                .insert(PathVisualMarker)
                                .insert(ChildOf(level_entity))
                                .id()
                        };

                        // Spawns a rectangle connecting start and end pos
                        let rectangle_entity_id = spawn_as_child_and_get_id((
                            Mesh3d(site_assets.robot_path_rectangle_mesh.clone()),
                            MeshMaterial3d(material.handle.clone()),
                            rectangle_tf,
                            visibility,
                        ));

                        // Spawns two circles, at start and end pos
                        let mut spawn_circle_fn = |tf| {
                            spawn_as_child_and_get_id((
                                Mesh3d(site_assets.robot_path_circle_mesh.clone()),
                                MeshMaterial3d(material.handle.clone()),
                                tf,
                                visibility,
                            ))
                        };

                        let start_circle_entity_id = spawn_circle_fn(start_circle_tf);
                        let end_circle_entity_id = spawn_circle_fn(end_circle_tf);

                        debug_data.rectangle_entities.push(rectangle_entity_id);
                        debug_data.circle_entities.push(start_circle_entity_id);
                        debug_data.circle_entities.push(end_circle_entity_id);
                    }
                }
            }

            for (_, robot_entity) in entity_id_map.iter() {
                // Inserts original poses for each robot if no original pose
                if let Some((_, _, pose, opose)) = robots.get(*robot_entity).ok() {
                    if opose.is_none() {
                        commands
                            .entity(*robot_entity)
                            .insert(Original::<Pose>(*pose));
                    }
                }
            }
        }
        MAPFDebugInfo::InProgress { .. } => {}
        MAPFDebugInfo::Failed { .. } => {
            for (robot_entity, _, _, robot_opose) in robots.iter() {
                if let Some(opose) = robot_opose {
                    change_pose.write(Change::new(opose.0, robot_entity));
                    commands.entity(robot_entity).remove::<Original<Pose>>();
                }
            }
        }
    }
}

pub fn set_path_all_visible(
    debug_data: &mut ResMut<NegotiationDebugData>,
    path_mesh_visibilities: &mut Query<&mut Visibility, With<PathVisualMarker>>,
) {
    // Reset all start pointers
    debug_data.start_pointers =
        get_start_pointers_from_trajectory_lengths(&debug_data.trajectory_lengths);
    let total_active_path_mesh_entities: usize =
        debug_data.trajectory_lengths.iter().sum::<usize>() - debug_data.trajectory_lengths.len();
    for i in 0..debug_data.rectangle_entities.len() {
        let visibility = if i < total_active_path_mesh_entities {
            Visibility::Visible
        } else {
            Visibility::Hidden
        };

        let mesh_entities = [
            debug_data.rectangle_entities[i],
            debug_data.circle_entities[2 * i],
            debug_data.circle_entities[2 * i + 1],
        ];

        for mesh_entity in mesh_entities {
            if let Some(mut visibility_mut) = path_mesh_visibilities.get_mut(mesh_entity).ok() {
                *visibility_mut = visibility;
            }
        }
    }
}

fn handle_removed_plan_info(
    mut path_visibilities: Query<&mut Visibility, With<PathVisualMarker>>,
    mut removed_plan_info: RemovedComponents<MAPFDebugInfo>,
) {
    if !removed_plan_info.is_empty() {
        for mut visibility in path_visibilities.iter_mut() {
            *visibility = Visibility::Hidden;
        }
        removed_plan_info.clear();
    }
}
