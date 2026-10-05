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
use rmf_site_msgs::nav_msgs::msg::Path;
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
use super::planned_paths::LivePathsState;

const LOCAL_PATH_COLOR: Color = Color::srgb(1.0, 0.0, 0.0);

#[derive(Debug, Clone)]
pub struct LiveEventLocalPlan {
    name: String,
    waypoints: Vec<Vec3>,
}

impl LiveStreamHandler for LiveEventLocalPlan {
    fn spawn_stream(
        robot_name: String,
        client: ClientHandle,
        sender: UnboundedSender<Self>,
        connection_requested: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    ) {
        let topic_name = format!("/{}/inner/plan", robot_name);

        let task = async move {
            if let Ok(local_plan_sub) = client.subscribe::<Path>(&topic_name).await {
                run_subscription_loop(
                    local_plan_sub,
                    sender,
                    connection_requested,
                    connection_active,
                    |path_msg| {
                        let waypoints: Vec<Vec3> = path_msg
                            .poses
                            .iter()
                            .map(|wp| {
                                Vec3::new(
                                    wp.pose.position.x as f32,
                                    wp.pose.position.y as f32,
                                    ZLayer::LocalPath.to_z(),
                                )
                            })
                            .collect();

                        LiveEventLocalPlan {
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
            .get_resource::<LiveLocalPathsState>()
            .is_none_or(|s| s.0.is_empty())
        {
            return;
        }
        let _ = world.run_system_cached(|mut state: ResMut<LiveLocalPathsState>| {
            state.0.clear();
        });
    }
}

#[derive(Default, Resource)]
pub struct LiveLocalPathsState(pub HashMap<String, Vec<Vec3>>);

pub fn update_live_local_paths(
    mut channel: ResMut<VisualizationStreamChannel<LiveEventLocalPlan>>,
    mut path_state: ResMut<LiveLocalPathsState>,
    planned_path_state: Res<LivePathsState>,
    robot_map: Res<LiveRobotsState>,
    mut gizmos: Gizmos,
) {
    while let Ok(event) = channel.receiver.try_recv() {
        path_state.0.insert(event.name, event.waypoints);
    }

    for (name, waypoints) in path_state.0.iter() {
        if !robot_map.0.contains_key(name) || waypoints.len() <= 1 {
            continue;
        }
        if planned_path_state
            .0
            .get(name)
            .is_none_or(|path_data| path_data.is_completed())
        {
            continue;
        }

        gizmos.linestrip(waypoints.iter().copied(), LOCAL_PATH_COLOR);
    }
}
