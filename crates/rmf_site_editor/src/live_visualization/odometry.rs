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

use crate::interaction::billboard::Billboard;
use crate::site::SiteAssets;
use bevy::prelude::*;
use bevy_rich_text3d::*;
use rmf_site_format::{Angle, NameInSite, Pose, Rotation};
use rmf_site_msgs::nav_msgs::msg::Odometry;
use roslibrust::rosbridge::ClientHandle;
use std::collections::HashMap;
use std::num::NonZero;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use super::live_state::{LiveStreamState, LoadSiteStatus};
use super::network_client::{
    run_subscription_loop, spawn_network_task, LiveStreamHandler, VisualizationStreamChannel,
};

// TODO: This z-height is currently hardcoded for the robot models used in the demo site.
// If different robots are used later on, we can instead do a dynamic check for
// the height of the models used.
const ROBOT_BILLBOARD_Z_HEIGHT: f32 = 0.4;

#[derive(Debug, Clone)]
pub struct LiveEventOdom {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    yaw: f32,
}

impl LiveStreamHandler for LiveEventOdom {
    fn spawn_stream(
        robot_name: String,
        client: ClientHandle,
        sender: UnboundedSender<Self>,
        connection_requested: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    ) {
        let topic_name = format!("/{}/odom", robot_name);

        let task = async move {
            if let Ok(odom_sub) = client.subscribe::<Odometry>(&topic_name).await {
                run_subscription_loop(
                    odom_sub,
                    sender,
                    connection_requested,
                    connection_active,
                    |odom_msg| {
                        let pos = &odom_msg.pose.pose.position;
                        let q = &odom_msg.pose.pose.orientation;

                        // Convert 3D ROS orientation quaternion to 2D yaw angle around z-axis
                        let siny_cosp: f64 = 2.0 * (q.w * q.z + q.x * q.y);
                        let cosy_cosp: f64 = 1.0 - 2.0 * (q.y * q.y + q.z * q.z);
                        let yaw = siny_cosp.atan2(cosy_cosp) as f32;

                        LiveEventOdom {
                            name: robot_name.clone(),
                            x: pos.x as f32,
                            y: pos.y as f32,
                            z: pos.z as f32,
                            yaw,
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
            .get_resource::<LiveRobotsState>()
            .is_none_or(|m| m.0.is_empty())
        {
            return;
        }
        let _ = world.run_system_cached(
            |mut commands: Commands,
             mut robot_map: ResMut<LiveRobotsState>,
             live_robots: Query<Entity, With<LiveRobotMarker>>,
             billboards: Query<Entity, With<LiveRobotBillboard>>| {
                robot_map.0.clear();
                for entity in live_robots.iter() {
                    commands.entity(entity).remove::<LiveRobotMarker>();
                }
                for billboard_entity in billboards.iter() {
                    commands.entity(billboard_entity).despawn();
                }
            },
        );
    }
}

#[derive(Component)]
pub struct LiveRobotMarker;

#[derive(Component)]
pub struct LiveRobotBillboard;

#[derive(Default, Resource)]
pub struct LiveRobotsState(pub HashMap<String, Entity>);

pub fn update_live_robots(
    state: Res<LiveStreamState>,
    mut channel: ResMut<VisualizationStreamChannel<LiveEventOdom>>,
    mut commands: Commands,
    mut robot_map: ResMut<LiveRobotsState>,
    untracked: Query<(Entity, &NameInSite), Without<LiveRobotMarker>>,
    mut poses: Query<&mut Pose>,
    site_assets: Res<SiteAssets>,
) {
    while let Ok(event) = channel.receiver.try_recv() {
        // Find existing robot
        let robot = if let Some(&entity) = robot_map.0.get(&event.name) {
            Some(entity)
        } else {
            // Find matching NameInSite for new untracked robot
            let mut found_entity = None;
            for (entity, name_in_site) in untracked.iter() {
                if name_in_site.0 == event.name {
                    commands.entity(entity).insert(LiveRobotMarker);

                    robot_map.0.insert(event.name.clone(), entity);
                    add_robot_status_billboard(&mut commands, &site_assets, entity, &event.name);

                    info!("Found existing robot: {}", event.name);
                    found_entity = Some(entity);
                    break;
                }
            }
            found_entity
        };

        if let Some(entity) = robot {
            if let Ok(mut pose) = poses.get_mut(entity) {
                pose.trans = [event.x, event.y, event.z];
                pose.rot = Rotation::Yaw(Angle::Rad(event.yaw).match_variant(pose.rot.yaw()));
            }
        } else if state.site_loaded && state.load_site_status == LoadSiteStatus::None {
            warn!("No matching NameInSite found for: {}", event.name);
        }
    }
}

fn add_robot_status_billboard(
    commands: &mut Commands,
    site_assets: &Res<SiteAssets>,
    robot_entity: Entity,
    robot_name: &str,
) {
    let billboard_pivot = commands
        .spawn((
            Transform::from_xyz(0.0, 0.0, ROBOT_BILLBOARD_Z_HEIGHT),
            Visibility::Inherited,
            LiveRobotBillboard,
        ))
        .id();

    let text_billboard_entity = commands
        .spawn((
            Text3d::new(robot_name),
            Text3dStyling {
                size: 250.,
                weight: Weight(900),
                stroke: NonZero::new(15),
                color: Srgba::BLACK,
                stroke_color: Srgba::WHITE,
                world_scale: Some(Vec2::splat(0.1)),
                layer_offset: 0.001,
                align: TextAlign::Center,
                anchor: TextAnchor::CENTER,
                ..Default::default()
            },
            Mesh3d::default(),
            MeshMaterial3d(site_assets.text3d_material.clone()),
            Billboard {
                offset: Vec3::ZERO,
                hover_enabled: false,
            },
        ))
        .id();

    commands
        .entity(billboard_pivot)
        .add_child(text_billboard_entity);
    commands.entity(robot_entity).add_child(billboard_pivot);
}
