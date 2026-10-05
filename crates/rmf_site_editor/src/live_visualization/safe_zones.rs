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
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rmf_site_msgs::rmf_prototype_msgs::msg::SafeZone;
use roslibrust::rosbridge::ClientHandle;
use std::collections::HashMap;
use std::sync::atomic::AtomicBool;
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use super::network_client::{
    run_subscription_loop, spawn_network_task, LiveStreamHandler, VisualizationStreamChannel,
};
use super::odometry::LiveRobotsState;
use super::planned_paths::LivePathsState;

const SAFE_ZONE_SCALE: i32 = 8;
const SAFE_ZONE_Z_OFFSET: f32 = 0.045;
const SAFE_ZONE_RGBA: [u8; 4] = [0, 255, 0, 50];
const SAFE_ZONE_OUTLINE_RGBA: [u8; 4] = [0, 255, 0, 120];

#[derive(Debug, Clone)]
pub struct LiveEventSafeZone {
    name: String,
    resolution: f32,
    size_x: u32,
    size_y: u32,
    origin_x: f32,
    origin_y: f32,
    image_size: Extent3d,
    rgba_data: Vec<u8>,
}

impl LiveStreamHandler for LiveEventSafeZone {
    fn spawn_stream(
        robot_name: String,
        client: ClientHandle,
        sender: UnboundedSender<Self>,
        connection_requested: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    ) {
        let topic_name = format!("/{}/plan/safe_zone", robot_name);

        let task = async move {
            if let Ok(safezone_sub) = client
                .subscribe_transient_local::<SafeZone>(&topic_name)
                .await
            {
                run_subscription_loop(
                    safezone_sub,
                    sender,
                    connection_requested,
                    connection_active,
                    |safezone_msg| {
                        let size_x = safezone_msg.costmap.metadata.size_x;
                        let size_y = safezone_msg.costmap.metadata.size_y;
                        // Convert costmap array to image metadata
                        let (image_size, rgba_data) =
                            convert_costmap_to_texture(size_x, size_y, &safezone_msg.costmap.data);
                        LiveEventSafeZone {
                            name: robot_name.clone(),
                            resolution: safezone_msg.costmap.metadata.resolution,
                            size_x,
                            size_y,
                            origin_x: safezone_msg.costmap.metadata.origin.position.x as f32,
                            origin_y: safezone_msg.costmap.metadata.origin.position.y as f32,
                            image_size,
                            rgba_data,
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
            .get_resource::<LiveSafeZonesState>()
            .is_none_or(|s| s.0.is_empty())
        {
            return;
        }
        let _ = world.run_system_cached(
            |mut commands: Commands,
             mut safe_zones_state: ResMut<LiveSafeZonesState>,
             mut images: ResMut<Assets<Image>>,
             mut meshes: ResMut<Assets<Mesh>>,
             mut materials: ResMut<Assets<StandardMaterial>>,
             safe_zones: Query<(
                &LiveSafeZoneMarker,
                &Mesh3d,
                &MeshMaterial3d<StandardMaterial>,
            )>| {
                for (_, entity) in safe_zones_state.0.drain() {
                    if let Ok((marker, mesh3d, mat3d)) = safe_zones.get(entity) {
                        images.remove(&marker.image_handle);
                        meshes.remove(&mesh3d.0);
                        materials.remove(&mat3d.0);
                    }
                    if let Ok(mut cmds) = commands.get_entity(entity) {
                        cmds.despawn();
                    }
                }
            },
        );
    }
}

#[derive(Default, Resource)]
pub struct LiveSafeZonesState(HashMap<String, Entity>);

#[derive(Component)]
pub struct LiveSafeZoneMarker {
    name: String,
    image_handle: Handle<Image>,
}

pub fn update_live_safe_zones(
    mut channel: ResMut<VisualizationStreamChannel<LiveEventSafeZone>>,
    path_state: Res<LivePathsState>,
    robot_map: Res<LiveRobotsState>,
    mut commands: Commands,
    mut safe_zones_state: ResMut<LiveSafeZonesState>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut safe_zones: Query<(
        Entity,
        &mut LiveSafeZoneMarker,
        &mut Transform,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
) {
    // Get latest safe zone messages for each robot
    let mut latest_events = HashMap::new();
    while let Ok(event) = channel.receiver.try_recv() {
        latest_events.insert(event.name.clone(), event);
    }

    for event in latest_events.into_values() {
        // Scale image to in-world dimensions
        let physical_width = event.size_x as f32 * event.resolution;
        let physical_height = event.size_y as f32 * event.resolution;

        // Position image on floor plan
        let target_transform =
            get_safezone_target_position(&event, physical_width, physical_height);

        let image_size = event.image_size;
        let rgba_data = event.rgba_data;

        // If safe zone already exists for robot, update existing pixel data
        if let Some(&entity) = safe_zones_state.0.get(&event.name) {
            if let Ok((_, mut marker, mut transform, mut mesh3d, mut mat3d, _)) =
                safe_zones.get_mut(entity)
            {
                let size_matches = images
                    .get(&marker.image_handle)
                    .is_some_and(|img| img.texture_descriptor.size == image_size);

                if size_matches {
                    if let Some(image) = images.get_mut(&marker.image_handle) {
                        image.data = Some(rgba_data);
                        // Mark material as mutated so the updated texture is re-extracted
                        let _ = materials.get_mut(&mat3d.0);
                    }
                } else {
                    images.remove(&marker.image_handle);
                    meshes.remove(&mesh3d.0);
                    materials.remove(&mat3d.0);

                    let (new_image_handle, new_mesh, new_mat) = create_safezone_assets(
                        image_size,
                        rgba_data,
                        physical_width,
                        physical_height,
                        &mut images,
                        &mut meshes,
                        &mut materials,
                    );

                    marker.image_handle = new_image_handle;
                    *mesh3d = new_mesh;
                    *mat3d = new_mat;
                }

                *transform = target_transform;
            }
        } else {
            // If safe zone does not exist yet, create new image and mesh
            let (image_handle, mesh3d, mat3d) = create_safezone_assets(
                image_size,
                rgba_data,
                physical_width,
                physical_height,
                &mut images,
                &mut meshes,
                &mut materials,
            );

            let entity = commands
                .spawn((
                    LiveSafeZoneMarker {
                        name: event.name.clone(),
                        image_handle,
                    },
                    mesh3d,
                    mat3d,
                    target_transform,
                    Visibility::Hidden,
                ))
                .id();

            safe_zones_state.0.insert(event.name.clone(), entity);
        }
    }

    // Update visibility of each existing safe zone based on robot progress
    for (_, marker, _, _, _, mut visibility) in safe_zones.iter_mut() {
        let robot_exists = robot_map.0.contains_key(&marker.name);

        let has_arrived = path_state
            .0
            .get(&marker.name)
            .is_none_or(|path_data| path_data.is_completed());

        if has_arrived || !robot_exists {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    }
}

fn convert_costmap_to_texture(size_x: u32, size_y: u32, data: &[u8]) -> (Extent3d, Vec<u8>) {
    let (size_x, size_y) = (size_x as i32, size_y as i32);

    // Multiply costmap dimensions by constant scale factor to increase resolution
    let scaled_size_x = size_x * SAFE_ZONE_SCALE;
    let scaled_size_y = size_y * SAFE_ZONE_SCALE;

    let mut rgba_data = vec![0u8; (scaled_size_x * scaled_size_y * 4) as usize];

    let is_safe_space = |orig_x: i32, orig_y: i32| -> bool {
        if !(0..size_x).contains(&orig_x) || !(0..size_y).contains(&orig_y) {
            // If out of bounds, it is not free space
            false
        } else {
            // Checks if pixel is in safe space by mapping back to raw event data
            let data_idx = (orig_y * size_x + orig_x) as usize;
            data.get(data_idx).copied() == Some(0)
        }
    };

    let get_edge_offset = |subpixel: i32| -> i32 {
        if subpixel == 0 {
            -1
        } else if subpixel == SAFE_ZONE_SCALE - 1 {
            1
        } else {
            0
        }
    };

    for orig_y in 0..size_y {
        for orig_x in 0..size_x {
            if !is_safe_space(orig_x, orig_y) {
                continue;
            }

            let is_unsafe = |offset_x: i32, offset_y: i32| -> bool {
                (offset_x != 0 || offset_y != 0)
                    && !is_safe_space(orig_x + offset_x, orig_y + offset_y)
            };

            let base_x = orig_x * SAFE_ZONE_SCALE;
            let base_y = orig_y * SAFE_ZONE_SCALE;

            for subpixel_y in 0..SAFE_ZONE_SCALE {
                // Invert Y-axis as costmap data starts at bottom-left but GPU textures start at top-left
                let new_y = scaled_size_y - 1 - (base_y + subpixel_y);
                let row_offset = new_y * scaled_size_x;
                let edge_offset_y = get_edge_offset(subpixel_y);
                let is_y_border = is_unsafe(0, edge_offset_y);

                for subpixel_x in 0..SAFE_ZONE_SCALE {
                    let edge_offset_x = get_edge_offset(subpixel_x);
                    let is_border = is_y_border
                        || is_unsafe(edge_offset_x, 0)
                        || is_unsafe(edge_offset_x, edge_offset_y);

                    let pixel_idx = ((row_offset + base_x + subpixel_x) * 4) as usize;
                    let color = if is_border {
                        &SAFE_ZONE_OUTLINE_RGBA
                    } else {
                        &SAFE_ZONE_RGBA
                    };
                    rgba_data[pixel_idx..pixel_idx + 4].copy_from_slice(color);
                }
            }
        }
    }

    let image_size = Extent3d {
        width: scaled_size_x as u32,
        height: scaled_size_y as u32,
        depth_or_array_layers: 1,
    };

    (image_size, rgba_data)
}

fn get_safezone_target_position(
    event: &LiveEventSafeZone,
    physical_width: f32,
    physical_height: f32,
) -> Transform {
    // origin_x and origin_y represent the center of grid cell (0, 0) not its bottom-left corner,
    // requiring a half-cell shift to align the quad center
    let center_x = event.origin_x + (physical_width - event.resolution) / 2.0;
    let center_y = event.origin_y + (physical_height - event.resolution) / 2.0;

    Transform::from_xyz(center_x, center_y, SAFE_ZONE_Z_OFFSET)
}

fn create_safezone_assets(
    image_size: Extent3d,
    rgba_data: Vec<u8>,
    physical_width: f32,
    physical_height: f32,
    images: &mut Assets<Image>,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> (Handle<Image>, Mesh3d, MeshMaterial3d<StandardMaterial>) {
    let new_image = Image::new(
        image_size,
        TextureDimension::D2,
        rgba_data,
        TextureFormat::Rgba8UnormSrgb,
        RenderAssetUsages::MAIN_WORLD | RenderAssetUsages::RENDER_WORLD,
    );
    let image_handle = images.add(new_image);

    let mesh3d = Mesh3d(meshes.add(Rectangle::new(physical_width, physical_height)));

    let mat3d = MeshMaterial3d(materials.add(StandardMaterial {
        base_color_texture: Some(image_handle.clone()),
        alpha_mode: AlphaMode::Blend,
        unlit: true,
        ..default()
    }));

    (image_handle, mesh3d, mat3d)
}
