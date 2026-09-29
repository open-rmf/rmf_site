/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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

use super::demo_world::*;
use crate::live_visualization::LiveStreamState;
use crate::{site::LoadSite, AppState, Autoload, WorkspaceLoader};
use bevy::{app::AppExit, prelude::*, window::PrimaryWindow};
use bevy_egui::{egui, EguiContexts};
use std::sync::atomic::Ordering;

const MAIN_MENU_PADDING: f32 = 10.0;

fn egui_ui(
    mut egui_context: EguiContexts,
    mut _exit: EventWriter<AppExit>,
    mut workspace_loader: WorkspaceLoader,
    mut _app_state: ResMut<State<AppState>>,
    autoload: Option<ResMut<Autoload>>,
    primary_windows: Query<Entity, With<PrimaryWindow>>,
    mut live_stream_state: ResMut<LiveStreamState>,
) {
    if let Some(mut autoload) = autoload {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(filename) = autoload.filename.take() {
                let _ = workspace_loader.load_from_path(filename);
            }
        }
        return;
    }

    let Some(ctx) = primary_windows
        .single()
        .ok()
        .and_then(|w| egui_context.try_ctx_for_entity_mut(w))
    else {
        return;
    };

    egui::Window::new("Welcome!")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .fixed_size(egui::vec2(700.0, 500.0))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
        .show(ctx, |ui| {
            ui.add_space(MAIN_MENU_PADDING);
            ui.vertical_centered(|ui| {
                ui.heading("Welcome to The RMF Site Editor!");
            });
            ui.add_space(MAIN_MENU_PADDING);

            ui.columns(2, |columns| {
                egui::Frame::NONE
                    .inner_margin(MAIN_MENU_PADDING)
                    .show(&mut columns[0], |ui| {
                        ui.heading("Create a Site:");
                        ui.add_space(MAIN_MENU_PADDING);

                        ui.vertical_centered_justified(|ui| {
                            if ui.button("View demo map").clicked() {
                                workspace_loader.load_site(async move {
                                    LoadSite::from_data(&demo_office(), None)
                                });
                            }
                            ui.add_space(MAIN_MENU_PADDING * 0.5);

                            if ui.button("Open a file").clicked() {
                                workspace_loader.load_from_dialog();
                            }
                            ui.add_space(MAIN_MENU_PADDING * 0.5);

                            if ui.button("Create new file").clicked() {
                                workspace_loader.create_empty_from_dialog();
                            }
                        });
                    });

                egui::Frame::NONE
                    .inner_margin(MAIN_MENU_PADDING)
                    .show(&mut columns[1], |ui| {
                        ui.heading("Visualize from Stream:");
                        ui.add_space(MAIN_MENU_PADDING);

                        ui.horizontal(|ui| {
                            ui.label("ROSBridge WS URL:");
                            ui.text_edit_singleline(&mut live_stream_state.url);
                        });

                        ui.horizontal(|ui| {
                            ui.label("Site Data HTTP URL:");
                            ui.text_edit_singleline(&mut live_stream_state.site_url);
                        });

                        ui.add_space(MAIN_MENU_PADDING * 0.5);

                        let connection_initiated = live_stream_state
                            .connection_requested
                            .load(Ordering::Relaxed);

                        ui.vertical_centered_justified(|ui| {
                            if connection_initiated {
                                ui.label("Stream requested...");
                            } else if ui.button("Connect").clicked() {
                                live_stream_state
                                    .connection_requested
                                    .store(true, Ordering::Relaxed);
                            }
                        });
                    });

                let x = (columns[0].max_rect().right() + columns[1].max_rect().left()) * 0.5;
                let top = columns[0].min_rect().top().min(columns[1].min_rect().top())
                    + MAIN_MENU_PADDING;
                let bottom = columns[0]
                    .min_rect()
                    .bottom()
                    .max(columns[1].min_rect().bottom());
                let stroke = columns[0].visuals().widgets.noninteractive.bg_stroke;
                columns[0].painter().vline(x, top..=bottom, stroke);
            });

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.add_space(MAIN_MENU_PADDING);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(MAIN_MENU_PADDING);
                        if ui.button("  Exit  ").clicked() {
                            _exit.write(AppExit::Success);
                        }
                    });
                });
                ui.add_space(MAIN_MENU_PADDING);
            }
        });
}

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, egui_ui.run_if(in_state(AppState::MainMenu)));
    }
}
