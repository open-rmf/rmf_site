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
use crate::site::{line_stroke_transform, LANE_LAYER_START};
use bevy::ecs::hierarchy::ChildOf;
use bevy::math::prelude::Rectangle;

pub const DEFAULT_PATH_WIDTH: f32 = 0.2;

// TODO (Nielsen) : Gather all layers in layers.rs
pub const OFFSET: f32 = 0.002;
pub const ROBOT_PATH_LAYER_START: f32 = LANE_LAYER_START + OFFSET;

#[derive(Component)]
pub struct PathVisualMarker;

pub fn visualise_selected_node(
    mut commands: Commands,
    negotiation_task: Res<NegotiationTask>,
    debug_data: ResMut<NegotiationDebugData>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    path_visuals: Query<Entity, With<PathVisualMarker>>,
    mut meshes: ResMut<Assets<Mesh>>,
    robots: Query<&Affiliation<Entity>, With<Robot>>,
    robot_descriptions: Query<&CircleCollision, (With<ModelMarker>, With<Group>)>,
    current_level: Res<CurrentLevel>,
) {
    // Return unless complete
    let NegotiationTaskStatus::Complete {
        elapsed_time: _,
        solution,
        negotiation_history,
        entity_id_map,
        error_message: _,
        conflicting_endpoints: _,
    } = &negotiation_task.status
    else {
        return;
    };
    if !debug_data.is_changed() {
        return;
    }
    // Despawn visuals from previous negotiation task
    for path_visual in path_visuals.iter() {
        commands.entity(path_visual).despawn();
    }

    let Some(selected_node) = debug_data
        .selected_negotiation_node
        .and_then(|selected_id| {
            if negotiation_history.is_empty() {
                solution.clone()
            } else {
                negotiation_history
                    .iter()
                    .find(|node| node.id == selected_id)
                    .map(|node| node.clone())
            }
        })
    else {
        return;
    };

    let Some(level_entity) = current_level.0 else {
        return;
    };

    if debug_data.visualize_trajectories {
        for proposal in selected_node.proposals.iter() {
            let Some(entity_id) = entity_id_map.get(&proposal.0) else {
                warn!("Unable to find entity id in map");
                continue;
            };
            let Ok(affiliation) = robots.get(*entity_id) else {
                warn!("Unable to get robot entity's affiliation");
                continue;
            };
            let Some(description_entity) = affiliation.0 else {
                warn!("No model description entity found");
                continue;
            };

            let mut collision_radius = DEFAULT_PATH_WIDTH / 2.0;

            if let Ok(cc) = robot_descriptions.get(description_entity) {
                collision_radius = cc.radius;
            } else {
                warn!(
                    "No circle collision model found for robot's model description, using default value of {}",
                    collision_radius
                );
            }

            let lane_material = materials.add(StandardMaterial {
                base_color: Color::srgb(1.0, 0.0, 0.0),
                unlit: true,
                ..Default::default()
            });

            let translation_to_vec3 = |x: f32, y: f32| {
                return Vec3::new(x, y, ROBOT_PATH_LAYER_START);
            };

            // Draws robot start and goal position
            {
                let robot_start_pos = match proposal.1.meta.trajectory.first() {
                    Some(waypoint) => waypoint.position.translation,
                    None => continue,
                };
                let robot_goal_pos = match proposal.1.meta.trajectory.last() {
                    Some(waypoint) => waypoint.position.translation,
                    None => continue,
                };

                // TODO (Nielsen) : Convert translation directly to Vec3
                let robot_start_pos =
                    translation_to_vec3(robot_start_pos.x as f32, robot_start_pos.y as f32);
                let robot_goal_pos =
                    translation_to_vec3(robot_goal_pos.x as f32, robot_goal_pos.y as f32);

                let mut spawn_circle_mesh = |pos| {
                    commands
                        .spawn((
                            Mesh3d(meshes.add(Circle::new(collision_radius))),
                            MeshMaterial3d(lane_material.clone()),
                            Transform::from_translation(pos),
                            Visibility::default(),
                        ))
                        .insert(PathVisualMarker)
                        .insert(ChildOf(level_entity));
                };
                spawn_circle_mesh(robot_start_pos);
                spawn_circle_mesh(robot_goal_pos);
            }

            let mut spawn_path_mesh = |start_pos,
                                       end_pos,
                                       lane_material: Handle<StandardMaterial>,
                                       lane_mesh,
                                       circle_mesh,
                                       robot_width| {
                commands
                    .spawn((
                        Mesh3d(circle_mesh),
                        MeshMaterial3d(lane_material.clone()),
                        Transform::from_translation(start_pos),
                        Visibility::default(),
                    ))
                    .insert(PathVisualMarker)
                    .insert(ChildOf(level_entity));
                commands
                    .spawn((
                        Mesh3d(lane_mesh),
                        MeshMaterial3d(lane_material.clone()),
                        line_stroke_transform(&start_pos, &end_pos, robot_width),
                        Visibility::default(),
                    ))
                    .insert(PathVisualMarker)
                    .insert(ChildOf(level_entity));
            };

            for slice in proposal.1.meta.trajectory.windows(2) {
                let start_pos = slice[0].position.translation;
                let end_pos = slice[1].position.translation;
                let start_pos = translation_to_vec3(start_pos.x as f32, start_pos.y as f32);
                let end_pos = translation_to_vec3(end_pos.x as f32, end_pos.y as f32);

                let robot_width = collision_radius * 2.0;
                spawn_path_mesh(
                    start_pos,
                    end_pos,
                    lane_material.clone(),
                    meshes.add(Mesh::from(Rectangle::new(1.0, 1.0))),
                    meshes.add(Circle::new(collision_radius)),
                    robot_width,
                );
            }
        }
    }
}
