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

use super::*;
use crate::{
    layers::ZLayer,
    site::{line_stroke_transform, Change, SiteAssets},
    CurrentWorkspace,
};
use bevy::ecs::hierarchy::ChildOf;
use mapf::motion::{se2::WaypointSE2, Motion, TimeCmp};
use rmf_site_format::NameOfSite;

pub const DEFAULT_PATH_WIDTH: f32 = 0.2;

#[derive(Component)]
pub struct PathVisualMarker;

#[derive(Component)]
pub enum MAPFDebugInfo {
    Success {
        longest_plan_duration_s: f32,
        elapsed_time: Duration,
        solution: NegotiationNode,
        entity_id_map: HashMap<usize, Entity>,
        negotiation_history: Vec<NegotiationNode>,
    },
    InProgress {
        start_time: Instant,
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
    },
    Failed {
        error_message: Option<String>,
        entity_id_map: HashMap<usize, Entity>,
        negotiation_history: Vec<NegotiationNode>,
        conflicts: Vec<(Entity, Entity)>,
    },
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

pub fn visualise_selected_node(
    mut commands: Commands,
    mut debug_data: ResMut<NegotiationDebugData>,
    path_visuals: Query<Entity, With<PathVisualMarker>>,
    mut change_pose: EventWriter<Change<Pose>>,
    robots: Query<&Affiliation<Entity>, With<Robot>>,
    robot_descriptions: Query<&CircleCollision, (With<ModelMarker>, With<Group>)>,
    current_level: Res<CurrentLevel>,
    now: Res<Time>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
    mapf_info: Query<&MAPFDebugInfo>,
    debugger_settings: Res<DebuggerSettings>,
    mapf_debug_window: Res<MAPFDebugDisplay>,
    site_assets: Res<SiteAssets>,
    robot_materials: Query<&DebugMaterial, With<Robot>>,
) {
    if !mapf_debug_window.show {
        return;
    }

    let Some(site) = current_workspace.to_site(&open_sites) else {
        return;
    };

    let Some(plan_info) = mapf_info.get(site).ok() else {
        return;
    };

    let Some(level_entity) = current_level.0 else {
        return;
    };

    let MAPFDebugInfo::Success {
        longest_plan_duration_s,
        elapsed_time: _,
        solution,
        entity_id_map,
        negotiation_history: _,
    } = plan_info
    else {
        return;
    };

    // TODO(Nielsen): to optimize
    for e in path_visuals.iter() {
        commands.entity(e).despawn();
    }

    // Update current animation time
    debug_data.time += debugger_settings.playback_speed * now.delta_secs();

    if debug_data.time > *longest_plan_duration_s {
        debug_data.time = 0.0;
    }

    for proposal in solution.proposals.iter() {
        let Some(robot_entity) = entity_id_map.get(&proposal.0) else {
            warn!("Unable to query for robot's entity id in map");
            continue;
        };
        let Some(affiliation) = robots.get(*robot_entity).ok() else {
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

        let time_now = debug_data.time;

        // Move robot to current position in trajectory
        if let Ok(interp) = proposal
            .1
            .meta
            .trajectory
            .motion()
            .compute_position(&mapf::motion::TimePoint::from_secs_f32(time_now))
        {
            let robot_yaw = crate::ops::atan2(interp.rotation.im as f32, interp.rotation.re as f32);

            change_pose.write(Change::new(
                Pose {
                    trans: [
                        interp.translation.x as f32,
                        interp.translation.y as f32,
                        get_robot_z_from_time(time_now, *longest_plan_duration_s),
                    ],
                    rot: Quat::from_rotation_z(robot_yaw).into(),
                },
                *robot_entity,
            ));
        }

        let mut draw_path = |start_pos: Vec3, end_pos: Vec3| {
            let Some(material) = robot_materials.get(*robot_entity).ok() else {
                return;
            };

            // Draws a rectangle connecting start and end pos
            commands
                .spawn((
                    Mesh3d(site_assets.robot_path_rectangle_mesh.clone()),
                    MeshMaterial3d(material.handle.clone()),
                    line_stroke_transform(&start_pos, &end_pos, collision_model.radius * 2.0),
                    Visibility::default(),
                ))
                .insert(PathVisualMarker)
                .insert(ChildOf(level_entity));

            // Draws two circles, at start and end pos
            let mut draw_circle_fn = |pos| {
                let radius = collision_model.radius;
                commands
                    .spawn((
                        Mesh3d(site_assets.robot_path_circle_mesh.clone()),
                        MeshMaterial3d(material.handle.clone()),
                        Transform::from_translation(pos).with_scale([radius, radius, 1.0].into()),
                        Visibility::default(),
                    ))
                    .insert(PathVisualMarker)
                    .insert(ChildOf(level_entity));
            };

            draw_circle_fn(start_pos);
            draw_circle_fn(end_pos);
        };

        let waypoint_to_xyz = |tf: TimeCmp<WaypointSE2>| {
            let time = tf.time.as_secs_f32();
            Vec3::new(
                tf.position.translation.x as f32,
                tf.position.translation.y as f32,
                get_robot_z_from_time(time, *longest_plan_duration_s),
            )
        };

        for slice in proposal.1.meta.trajectory.windows(2) {
            // Do not draw if the path is old (expired)
            if time_now > slice[0].time.as_secs_f32() {
                continue;
            }

            let start_pos = waypoint_to_xyz(slice[0]);
            let end_pos = waypoint_to_xyz(slice[1]);

            draw_path(start_pos, end_pos);
        }
    }
}
