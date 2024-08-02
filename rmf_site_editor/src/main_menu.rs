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
use crate::{AppState, Autoload, WorkspaceData, WorkspaceLoadingServices};
use bevy::{app::AppExit, prelude::*, window::PrimaryWindow};
use bevy_egui::{egui, EguiContexts};

fn egui_ui(
    mut commands: Commands,
    mut egui_context: EguiContexts,
    mut _exit: EventWriter<AppExit>,
    load_workspace: Res<WorkspaceLoadingServices>,
    mut _app_state: ResMut<State<AppState>>,
    autoload: Option<ResMut<Autoload>>,
    primary_windows: Query<Entity, With<PrimaryWindow>>,
) {
    if let Some(mut autoload) = autoload {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(filename) = autoload.filename.take() {
                load_workspace.load_from_path(&mut commands, filename);
            }
        }
        return;
    }

    let Some(ctx) = primary_windows
        .get_single()
        .ok()
        .and_then(|w| egui_context.try_ctx_for_window_mut(w))
    else {
        return;
    };

    egui::Window::new("Welcome!")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
        .show(ctx, |ui| {
            ui.heading("Welcome to The RMF Site Editor!");
            ui.add_space(10.);

            ui.horizontal(|ui| {
                if ui.button("View demo map").clicked() {
                    load_workspace.load_from_data(
                        &mut commands,
                        WorkspaceData::LegacyBuilding(demo_office()),
                    );
                }

                if ui.button("Open a file").clicked() {
                    load_workspace.load_from_dialog(&mut commands);
                }

                if ui.button("Create new file").clicked() {
                    load_workspace.create_empty_from_dialog(&mut commands);
                }

                // TODO(@mxgrey): Bring this back when we have finished developing
                // the key features for workcell editing.
                // if ui.button("Workcell Editor").clicked() {
                //     load_workspace.send(LoadWorkspace::Data(WorkspaceData::Workcell(
                //         demo_workcell(),
                //     )));
                // }

                // TODO(@mxgrey): Bring this back when we have time to fix the
                // warehouse generator.
                // if ui.button("Warehouse generator").clicked() {
                //     info!("Entering warehouse generator");
                //     _app_state.overwrite_set(AppState::WarehouseGenerator).unwrap();
                // }
            });

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.add_space(20.);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Exit").clicked() {
                            _exit.send(AppExit);
                        }
                    });
                });
            }
        });
}

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, egui_ui.run_if(in_state(AppState::MainMenu)));
    }
}
