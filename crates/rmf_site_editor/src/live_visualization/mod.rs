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

mod live_state;
mod network_client;
mod odometry;
mod planned_paths;
mod safe_zones;

use bevy::prelude::*;
use rmf_site_egui::{HeaderPanel, HeaderTilePlugin};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;

pub use live_state::LiveStreamState;
use live_state::{
    auto_fetch_site_on_connect, check_load_site_completion, load_site_status_ui,
    process_site_download, LiveStreamStatusWidget, LoadSiteStatus, SiteFetchReceiver,
};
use network_client::{start_rosbridge_subscriber, StreamPlugin, StreamRegistry};
use odometry::{update_live_robots, LiveEventOdom, LiveRobotsState};
use planned_paths::{update_live_paths, LiveEventPlan, LiveEventProgress, LivePathsState};
use safe_zones::{update_live_safe_zones, LiveEventSafeZone, LiveSafeZonesState};

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
        .init_resource::<LiveRobotsState>()
        .init_resource::<LivePathsState>()
        .init_resource::<LiveSafeZonesState>()
        .add_systems(
            Update,
            start_live_stream.run_if(in_state(crate::AppState::MainMenu)),
        )
        .add_systems(
            Update,
            (
                auto_fetch_site_on_connect,
                process_site_download,
                load_site_status_ui,
                check_load_site_completion,
            )
                .run_if(in_state(crate::AppState::SiteStream)),
        )
        .add_systems(
            Update,
            (
                update_live_robots,
                update_live_paths,
                update_live_safe_zones,
            )
                .run_if(in_state(crate::AppState::SiteStream))
                .run_if(LiveStreamState::in_connected_mode()),
        )
        .add_systems(OnExit(crate::AppState::SiteStream), disconnect_live_stream);

        if app.world().get_resource::<HeaderPanel>().is_some() {
            app.add_plugins(HeaderTilePlugin::<LiveStreamStatusWidget>::new());
        }
    }
}

fn start_live_stream(
    mut live_stream_state: ResMut<LiveStreamState>,
    registry: Res<StreamRegistry>,
    mut next_app_state: ResMut<NextState<crate::AppState>>,
    mut next_interaction_state: ResMut<NextState<crate::interaction::InteractionState>>,
    mut load_site: EventWriter<crate::site::LoadSite>,
) {
    if live_stream_state.is_streaming() {
        live_stream_state.connection_requested = Arc::new(AtomicBool::new(true));
        live_stream_state.connection_active = Arc::new(AtomicBool::new(false));

        start_rosbridge_subscriber(
            &live_stream_state.url,
            registry.clone(),
            live_stream_state.connection_requested.clone(),
            live_stream_state.connection_active.clone(),
        );

        next_app_state.set(crate::AppState::SiteStream);
        next_interaction_state.set(crate::interaction::InteractionState::Enable);
        load_site.write(crate::site::LoadSite::blank_L1("live".to_owned(), None));
    }
}

fn disconnect_live_stream(mut commands: Commands, mut state: ResMut<LiveStreamState>) {
    commands.remove_resource::<SiteFetchReceiver>();
    state.connection_requested.store(false, Ordering::Relaxed);
    state.connection_active.store(false, Ordering::Relaxed);
    state.site_loaded = false;
    state.load_site_status = LoadSiteStatus::None;
    state.retry_timer = None;
}
