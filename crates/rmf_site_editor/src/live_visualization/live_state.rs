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

use crate::site::{LoadSite, ModelFailedLoading, PendingModel};
use bevy::ecs::system::{SystemParam, SystemState};
use bevy::prelude::*;
use bevy_egui::{egui, EguiContexts};
use rmf_site_egui::{Tile, WidgetSystem};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use tokio::sync::mpsc::error::TryRecvError;
use tokio::sync::mpsc::UnboundedReceiver;

const DEFAULT_CONNECTION_URL: &str = "ws://127.0.0.1:9090";
const DEFAULT_SITE_DATA_URL: &str = "http://127.0.0.1:8080/site_file";

#[derive(Default, Clone, Copy, PartialEq, Eq)]
pub enum LoadSiteStatus {
    #[default]
    None,
    Receiving,
    Loading,
}

// Holds receiver to receive site data asynchronously
#[derive(Resource)]
pub struct SiteFetchReceiver(UnboundedReceiver<Result<LoadSite, String>>);

#[derive(Resource)]
pub struct LiveStreamState {
    pub url: String,
    pub site_url: String,
    pub connection_requested: Arc<AtomicBool>,
    pub connection_active: Arc<AtomicBool>,
    pub site_loaded: bool,
    pub load_site_status: LoadSiteStatus,
}

impl Default for LiveStreamState {
    fn default() -> Self {
        Self {
            url: DEFAULT_CONNECTION_URL.to_string(),
            site_url: DEFAULT_SITE_DATA_URL.to_string(),
            connection_requested: Arc::new(AtomicBool::new(false)),
            connection_active: Arc::new(AtomicBool::new(false)),
            site_loaded: false,
            load_site_status: LoadSiteStatus::None,
        }
    }
}

#[derive(SystemParam)]
pub struct LiveStreamStatusWidget<'w> {
    state: Res<'w, LiveStreamState>,
}

impl<'w> WidgetSystem<Tile> for LiveStreamStatusWidget<'w> {
    fn show(_: Tile, ui: &mut egui::Ui, state: &mut SystemState<Self>, world: &mut World) {
        let params = state.get(world);

        if !params.state.connection_requested.load(Ordering::Relaxed) {
            return;
        }

        ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
            if params.state.connection_active.load(Ordering::Relaxed) {
                ui.label(egui::RichText::new("\u{2022}  Connected").color(egui::Color32::GREEN));
            } else {
                ui.label(egui::RichText::new("\u{2022}  Disconnected").color(egui::Color32::RED));
            }
        });
    }
}

pub fn auto_fetch_site_on_connect(mut state: ResMut<LiveStreamState>, mut commands: Commands) {
    let is_currently_active = state.connection_active.load(Ordering::Relaxed);

    if is_currently_active && !state.site_loaded {
        state.site_loaded = true;
        state.load_site_status = LoadSiteStatus::Receiving;

        let (tx, rx) = tokio::sync::mpsc::unbounded_channel();
        commands.insert_resource(SiteFetchReceiver(rx));

        let request = ehttp::Request::get(&state.site_url);
        ehttp::fetch(request, move |result| {
            if let Ok(response) = result {
                if response.status == 200 {
                    let parsed =
                        LoadSite::from_data(&response.bytes, None).map_err(|e| e.to_string());
                    let _ = tx.send(parsed);
                }
            }
        });
    }
}

// Spawns site from data once all site data bytes are ready
pub fn process_site_download(
    mut commands: Commands,
    receiver: Option<ResMut<SiteFetchReceiver>>,
    mut state: ResMut<LiveStreamState>,
    mut load_site: EventWriter<LoadSite>,
) {
    if let Some(mut rx) = receiver {
        match rx.0.try_recv() {
            Ok(Ok(mut site)) => {
                state.load_site_status = LoadSiteStatus::Loading;
                site.focus = true;
                load_site.write(site);
                commands.remove_resource::<SiteFetchReceiver>();
            }
            Ok(Err(_)) | Err(TryRecvError::Disconnected) => {
                state.load_site_status = LoadSiteStatus::None;
                commands.remove_resource::<SiteFetchReceiver>();
            }
            Err(TryRecvError::Empty) => {}
        }
    }
}

pub fn load_site_status_ui(state: Res<LiveStreamState>, mut egui_context: EguiContexts) {
    let text = match state.load_site_status {
        LoadSiteStatus::Receiving => "Receiving site data...",
        LoadSiteStatus::Loading => "Loading site data...",
        LoadSiteStatus::None => return,
    };

    egui::Window::new("Site Load Status")
        .title_bar(false)
        .resizable(false)
        .collapsible(false)
        .anchor(egui::Align2::LEFT_TOP, [10.0, 40.0])
        .show(egui_context.ctx_mut(), |ui| {
            egui::Frame::NONE.inner_margin(4.0).show(ui, |ui| {
                ui.horizontal(|ui| {
                    ui.add(egui::Spinner::new().size(8.0).color(egui::Color32::WHITE));
                    ui.label(
                        egui::RichText::new(text)
                            .size(12.0)
                            .color(egui::Color32::WHITE),
                    );
                });
            });
        });
}

pub fn check_load_site_completion(
    mut state: ResMut<LiveStreamState>,
    load_site_events: EventReader<LoadSite>,
    pending_models: Query<(), (With<PendingModel>, Without<ModelFailedLoading>)>,
) {
    if state.load_site_status == LoadSiteStatus::Loading
        && load_site_events.is_empty()
        && pending_models.is_empty()
    {
        state.load_site_status = LoadSiteStatus::None;
    }
}
