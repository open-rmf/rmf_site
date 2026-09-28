use bevy::prelude::*;
use bevy::render::render_asset::RenderAssetUsages;
use bevy::render::render_resource::{Extent3d, TextureDimension, TextureFormat};
use rmf_site_msgs::rmf_prototype_msgs::msg::SafeZone;
use roslibrust::rosbridge::ClientHandle;
use std::collections::HashMap;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::UnboundedSender;

use super::live_state::LiveStreamState;
use super::network_client::{
    spawn_network_task, wait_until_inactive, LiveStreamHandler, VisualizationStreamChannel,
};
use super::odometry::LiveRobotMarker;
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
    data: Vec<u8>,
}

impl LiveStreamHandler for LiveEventSafeZone {
    fn spawn_stream(
        robot_name: String,
        client: ClientHandle,
        sender: UnboundedSender<Self>,
        connect_flag: Arc<AtomicBool>,
        connection_active: Arc<AtomicBool>,
    ) {
        let topic_name = format!("/{}/plan/safe_zone", robot_name);

        let task = async move {
            if let Ok(sz_sub) = client
                .subscribe_transient_local::<SafeZone>(&topic_name)
                .await
            {
                loop {
                    let sz_msg = tokio::select! {
                        msg = sz_sub.next() => msg,
                        _ = wait_until_inactive(&connection_active) => break,
                    };

                    if !connect_flag.load(Ordering::Relaxed)
                        || !connection_active.load(Ordering::Relaxed)
                    {
                        break;
                    }

                    if let Err(e) = sender.send(LiveEventSafeZone {
                        name: robot_name.clone(),
                        resolution: sz_msg.costmap.metadata.resolution,
                        size_x: sz_msg.costmap.metadata.size_x,
                        size_y: sz_msg.costmap.metadata.size_y,
                        origin_x: sz_msg.costmap.metadata.origin.position.x as f32,
                        origin_y: sz_msg.costmap.metadata.origin.position.y as f32,
                        data: sz_msg.costmap.data,
                    }) {
                        error!("Failed to send SafeZone event across channel: {}", e);
                        break;
                    }
                }
            }
        };
        spawn_network_task(task);
    }
}

#[derive(Default, Resource)]
pub struct LiveSafeZoneState(pub HashMap<String, Entity>);

#[derive(Component)]
pub struct SafeZoneMarker {
    name: String,
    image_handle: Handle<Image>,
}

pub fn update_live_safe_zones(
    state: Res<LiveStreamState>,
    mut channel: ResMut<VisualizationStreamChannel<LiveEventSafeZone>>,
    path_state: Res<LivePathsState>,
    mut commands: Commands,
    mut safe_zones_state: ResMut<LiveSafeZoneState>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut marker_query: Query<(
        Entity,
        &mut SafeZoneMarker,
        &mut Transform,
        &mut Mesh3d,
        &mut MeshMaterial3d<StandardMaterial>,
        &mut Visibility,
    )>,
    robot_query: Query<&LiveRobotMarker>,
) {
    // Cleanup all SafeZone entities on network disconnect
    if !state.connection_active.load(Ordering::Relaxed) {
        for (_, entity) in safe_zones_state.0.drain() {
            if let Ok(mut cmds) = commands.get_entity(entity) {
                cmds.despawn();
            }
        }
        return;
    }

    // Get latest SafeZone messages for each robot
    let mut latest_events = HashMap::new();
    while let Ok(event) = channel.receiver.try_recv() {
        latest_events.insert(event.name.clone(), event);
    }

    // Convert costmap array to image metadata
    for (_, event) in latest_events {
        let (image_size, rgba_data) = convert_costmap_to_texture(&event);

        // Scale image to in-world dimensions
        let physical_width = event.size_x as f32 * event.resolution;
        let physical_height = event.size_y as f32 * event.resolution;

        // Position image on floor plan
        let target_transform =
            get_safezone_target_position(&event, physical_width, physical_height);

        // If SafeZone already exists for robot, update existing pixel data
        if let Some(&entity) = safe_zones_state.0.get(&event.name) {
            if let Ok((_, mut marker, mut transform, mut mesh3d, mut mat3d, _)) =
                marker_query.get_mut(entity)
            {
                let size_matches = images
                    .get(&marker.image_handle)
                    .map_or(false, |img| img.texture_descriptor.size == image_size);

                if size_matches {
                    if let Some(image) = images.get_mut(&marker.image_handle) {
                        image.data = Some(rgba_data);
                        let _ = materials.get_mut(&mat3d.0);
                    }
                } else {
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
            // If SafeZone does not exist yet, create new image and mesh
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
                    SafeZoneMarker {
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

    // Update visibility of each existing SafeZones based on robot progress
    for (_, marker, _, _, _, mut visibility) in marker_query.iter_mut() {
        let mut robot_exists = false;
        for robot in robot_query.iter() {
            if robot.name == marker.name {
                robot_exists = true;
                break;
            }
        }

        let has_arrived = path_state
            .0
            .get(&marker.name)
            .map_or(true, |path_data| path_data.is_completed());

        if has_arrived || !robot_exists {
            *visibility = Visibility::Hidden;
        } else {
            *visibility = Visibility::Inherited;
        }
    }
}

fn convert_costmap_to_texture(event: &LiveEventSafeZone) -> (Extent3d, Vec<u8>) {
    // Multiply costmap dimensions by constant scale factor to increase resolution
    let scaled_size_x = (event.size_x as i32) * SAFE_ZONE_SCALE;
    let scaled_size_y = (event.size_y as i32) * SAFE_ZONE_SCALE;

    let mut rgba_data = vec![0u8; (scaled_size_x * scaled_size_y * 4) as usize];

    let is_safe_space = |scaled_x: i32, scaled_y: i32| -> bool {
        if scaled_x < 0 || scaled_x >= scaled_size_x || scaled_y < 0 || scaled_y >= scaled_size_y {
            // If out of bounds, it is not free space
            false
        } else {
            // Checks if pixel is in safe space by mapping back to raw event data
            let orig_x = (scaled_x / SAFE_ZONE_SCALE) as usize;
            let orig_y = (scaled_y / SAFE_ZONE_SCALE) as usize;
            let ros_idx = orig_y * (event.size_x as usize) + orig_x;
            event.data[ros_idx] == 0
        }
    };

    let is_border = |x: i32, y: i32| -> bool {
        for ny in (y - 1)..=(y + 1) {
            for nx in (x - 1)..=(x + 1) {
                if !is_safe_space(nx, ny) {
                    return true;
                }
            }
        }
        false
    };

    for y in 0..scaled_size_y {
        for x in 0..scaled_size_x {
            if is_safe_space(x, y) {
                let new_y = scaled_size_y - 1 - y;
                let pixel_idx = (new_y * scaled_size_x + x) as usize * 4;

                if is_border(x, y) {
                    rgba_data[pixel_idx..pixel_idx + 4].copy_from_slice(&SAFE_ZONE_OUTLINE_RGBA);
                } else {
                    rgba_data[pixel_idx..pixel_idx + 4].copy_from_slice(&SAFE_ZONE_RGBA);
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
