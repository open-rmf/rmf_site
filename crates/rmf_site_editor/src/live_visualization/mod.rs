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

pub mod live_state;
pub mod network_client;
mod odometry;
mod planned_paths;
mod safe_zones;

use bevy::prelude::*;
use rmf_site_egui::{HeaderPanel, HeaderTilePlugin};
use std::sync::atomic::Ordering;

use live_state::{
    auto_fetch_site_on_connect, check_load_site_completion, load_site_status_ui,
    process_site_download, LiveStreamState, LiveStreamStatusWidget,
};
use network_client::StreamPlugin;
use odometry::{update_live_robots, LiveEventOdom, LiveRobotMarker, LiveRobotsMap};
use planned_paths::{update_live_paths, LiveEventPlan, LiveEventProgress, LivePathsState};
use safe_zones::{update_live_safe_zones, LiveEventSafeZone, LiveSafeZoneState, SafeZoneMarker};

pub struct LiveVisualizationPlugin;

impl Plugin for LiveVisualizationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            StreamPlugin::<LiveEventOdom>::default(),
            StreamPlugin::<LiveEventPlan>::default(),
            StreamPlugin::<LiveEventProgress>::default(),
            StreamPlugin::<LiveEventSafeZone>::default(),
        ))
        .init_resource::<LiveStreamState>()
        .init_resource::<LiveRobotsMap>()
        .init_resource::<LivePathsState>()
        .init_resource::<LiveSafeZoneState>()
        .add_systems(
            Update,
            (
                update_live_robots,
                update_live_paths,
                update_live_safe_zones,
                auto_fetch_site_on_connect,
                process_site_download,
                load_site_status_ui,
                check_load_site_completion,
            )
                .run_if(in_state(crate::AppState::SiteEditor)),
        )
        .add_systems(OnEnter(crate::AppState::MainMenu), disconnect_live_stream);

        if app.world().get_resource::<HeaderPanel>().is_some() {
            app.add_plugins(HeaderTilePlugin::<LiveStreamStatusWidget>::new());
        }
    }
}

fn disconnect_live_stream(
    mut commands: Commands,
    mut state: ResMut<LiveStreamState>,
    mut robot_map: ResMut<LiveRobotsMap>,
    mut path_state: ResMut<LivePathsState>,
    mut safe_zones_state: ResMut<LiveSafeZoneState>,
    mut images: ResMut<Assets<Image>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    live_robots: Query<Entity, With<LiveRobotMarker>>,
    safe_zones: Query<(
        Entity,
        &SafeZoneMarker,
        &Mesh3d,
        &MeshMaterial3d<StandardMaterial>,
    )>,
) {
    state.connection_requested.store(false, Ordering::Relaxed);
    state.connection_active.store(false, Ordering::Relaxed);
    state.site_loaded = false;
    state.load_site_status = crate::live_visualization::live_state::LoadSiteStatus::None;
    robot_map.0.clear();
    path_state.0.clear();
    safe_zones_state.0.clear();
    for entity in live_robots.iter() {
        commands.entity(entity).despawn();
    }
    for (entity, marker, mesh3d, mat3d) in safe_zones.iter() {
        images.remove(&marker.image_handle);
        meshes.remove(&mesh3d.0);
        materials.remove(&mat3d.0);
        commands.entity(entity).despawn();
    }
}
