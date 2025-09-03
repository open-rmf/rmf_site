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
use crate::{site::Change, CurrentWorkspace};
use mapf::motion::Motion;
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
    },
    Failed {
        error_message: Option<String>,
        entity_id_map: HashMap<usize, Entity>,
        negotiation_history: Vec<NegotiationNode>,
        conflicts: Vec<(Entity, Entity)>,
    },
}

pub fn visualise_selected_node(
    mut debug_data: ResMut<NegotiationDebugData>,
    mut change_pose: EventWriter<Change<Pose>>,
    now: Res<Time>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_workspace: Res<CurrentWorkspace>,
    mapf_info: Query<&MAPFDebugInfo>,
    debugger_settings: Res<DebuggerSettings>,
    mapf_debug_window: Res<MAPFDebugDisplay>,
    mut path_mesh_visibilities: Query<&mut Visibility, With<PathVisualMarker>>,
    mut set_all_path_visible_request: EventWriter<SetAllPathVisibleRequest>,
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

    // Update current animation time
    debug_data.time += debugger_settings.playback_speed * now.delta_secs();

    if debug_data.time > *longest_plan_duration_s {
        set_all_path_visible_request.write(SetAllPathVisibleRequest);
        debug_data.time = 0.0;
        return;
    }

    let original_start_pointers =
        get_start_pointers_from_trajectory_lengths(&debug_data.trajectory_lengths);

    for (i, proposal) in solution.proposals.iter().enumerate() {
        let Some(robot_entity) = entity_id_map.get(&proposal.0) else {
            warn!("Unable to query for robot's entity id in map");
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

        let mut cur_ptr = if let Some(cur_ptr) = debug_data.start_pointers.get(i) {
            *cur_ptr
        } else {
            continue;
        };

        let original_ptr = original_start_pointers[i];

        while (cur_ptr - original_ptr) < proposal.1.meta.trajectory.len() - 1 {
            let cur_mesh_time = proposal.1.meta.trajectory[cur_ptr - original_ptr]
                .time
                .as_secs_f32();
            if time_now < cur_mesh_time {
                break;
            }
            let Some(rect_entity) = debug_data.rectangle_entities.get(cur_ptr) else {
                error!("Unable to get rectangle entity");
                break;
            };
            let start_circle_entity = debug_data.circle_entities[2 * cur_ptr];
            let end_circle_entity = debug_data.circle_entities[2 * cur_ptr + 1];
            for mesh_entity in [*rect_entity, start_circle_entity, end_circle_entity] {
                if let Some(mut visibility_mut) = path_mesh_visibilities.get_mut(mesh_entity).ok() {
                    *visibility_mut = Visibility::Hidden;
                }
            }
            cur_ptr += 1;
            debug_data.start_pointers[i] += 1;
        }
    }
}
