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
use rmf_site_msgs::rmf_prototype_msgs::msg::{
    DestinationConstraints, DestinationGoal, Region, TargetRegion,
};
use rmf_site_msgs::unique_identifier_msgs::msg::UUID;
use roslibrust::rosbridge::{ClientHandle, Publisher};
use std::collections::{hash_map::Entry, HashMap};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::{unbounded_channel, UnboundedReceiver, UnboundedSender};

use super::destination_picking::RobotDestination;

impl RobotDestination {
    pub fn to_destination_goal(&self) -> DestinationGoal {
        let target_region = TargetRegion {
            tolerance: 0.0,
            region: Region {
                points: vec![self.position.x, self.position.y],
                hint: Region::HINT_POINT,
            },
            orientations: Vec::new(),
        };

        DestinationGoal {
            one_of: vec![DestinationConstraints {
                regions: vec![target_region],
                nodes: self.target_nodes.clone(),
            }],
            cost_bias: vec![0.0],
            session: UUID {
                uuid: uuid::Uuid::new_v4().into_bytes(),
            },
        }
    }
}

#[derive(Resource, Clone)]
pub struct DestinationGoalSender {
    sender: UnboundedSender<(String, DestinationGoal)>,
}

impl Default for DestinationGoalSender {
    fn default() -> Self {
        let (sender, _) = unbounded_channel();
        Self { sender }
    }
}

impl DestinationGoalSender {
    pub fn new_channel() -> (Self, DestinationGoalReceiver) {
        let (sender, receiver) = unbounded_channel();
        (Self { sender }, DestinationGoalReceiver::new(receiver))
    }

    pub fn publish(&self, robot_name: String, goal: DestinationGoal) {
        if let Err(e) = self.sender.send((robot_name, goal)) {
            error!("Failed to queue DestinationGoal for publishing: {}", e);
        }
    }
}

pub struct DestinationGoalReceiver {
    receiver: UnboundedReceiver<(String, DestinationGoal)>,
    goal_publishers: HashMap<String, Publisher<DestinationGoal>>,
}

impl DestinationGoalReceiver {
    fn new(receiver: UnboundedReceiver<(String, DestinationGoal)>) -> Self {
        Self {
            receiver,
            goal_publishers: HashMap::new(),
        }
    }

    pub async fn run_publish_loop(
        &mut self,
        client: &ClientHandle,
        connection_requested: &Arc<AtomicBool>,
        connection_active: &Arc<AtomicBool>,
    ) {
        self.goal_publishers.clear();

        while let Some((robot_name, goal)) = self.receiver.recv().await {
            if !connection_requested.load(Ordering::Relaxed)
                || !connection_active.load(Ordering::Relaxed)
            {
                break;
            }

            self.publish(client, &robot_name, &goal).await;
        }
    }

    async fn publish(&mut self, client: &ClientHandle, robot_name: &str, goal: &DestinationGoal) {
        let publisher = match self.goal_publishers.entry(robot_name.to_string()) {
            Entry::Occupied(entry) => entry.into_mut(),
            Entry::Vacant(entry) => {
                let topic = format!("/{}/destination/goal", robot_name);
                match client.advertise::<DestinationGoal>(&topic).await {
                    Ok(publisher) => entry.insert(publisher),
                    Err(e) => {
                        error!("Failed to advertise {}: {}", topic, e);
                        return;
                    }
                }
            }
        };

        if let Err(e) = publisher.publish(goal).await {
            error!(
                "Failed to publish DestinationGoal for {}: {}",
                robot_name, e
            );
        }
    }
}
