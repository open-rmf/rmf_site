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
use rmf_site_msgs::rmf_prototype_msgs::msg::Destination;
use roslibrust::rosbridge::ClientHandle;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use super::network_client::{
    run_subscription_loop, spawn_network_task, LiveStreamHandler, VisualizationStreamChannel,
};

#[derive(Debug, Clone)]
pub struct LiveEventDestination {
    name: String,
    is_detour: bool,
}

impl LiveStreamHandler for LiveEventDestination {
    fn spawn_stream(
        robot_name: String,
        client: ClientHandle,
        sender: UnboundedSender<Self>,
        connection_requested: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    ) {
        let topic_name = format!("/{}/destination", robot_name);

        let task = async move {
            if let Ok(dest_sub) = client
                .subscribe_transient_local::<Destination>(&topic_name)
                .await
            {
                run_subscription_loop(
                    dest_sub,
                    sender,
                    connection_requested,
                    connection_active,
                    |dest_msg| {
                        // `detour_for_goal` is zero-initialized unless this destination is a
                        // temporary detour for an active goal session
                        let is_detour = dest_msg.detour_for_goal.uuid != [0u8; 16];
                        LiveEventDestination {
                            name: robot_name.clone(),
                            is_detour,
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
            .get_resource::<LiveDestinationsState>()
            .is_none_or(|s| s.detours.is_empty())
        {
            return;
        }
        let _ = world.run_system_cached(|mut state: ResMut<LiveDestinationsState>| {
            state.detours.clear();
        });
    }
}

#[derive(Default, Resource)]
pub struct LiveDestinationsState {
    pub detours: HashMap<String, bool>,
}

pub fn update_live_destinations(
    mut channel: ResMut<VisualizationStreamChannel<LiveEventDestination>>,
    mut destination_state: ResMut<LiveDestinationsState>,
) {
    while let Ok(event) = channel.receiver.try_recv() {
        destination_state
            .detours
            .insert(event.name, event.is_detour);
    }
}
