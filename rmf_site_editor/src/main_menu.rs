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
use crate::{site::send_load_site_event, site::LoadSite, AppState, LoadWorkspace};
use bevy::{app::AppExit, prelude::*};
use bevy_egui::{egui, EguiContexts};
use rmf_site_format::legacy::building_map::BuildingMap;
use std::path::PathBuf;

#[derive(Resource)]
pub struct Autoload {
    pub filename: Option<PathBuf>,
    pub import: Option<PathBuf>,
}

impl Autoload {
    pub fn file(filename: PathBuf, import: Option<PathBuf>) -> Self {
        Autoload {
            filename: Some(filename),
            import,
        }
    }
}

fn egui_ui(
    mut egui_context: EguiContexts,
    mut exit: EventWriter<AppExit>,
    mut load_site: EventWriter<LoadSite>,
    mut load_workspace: EventWriter<LoadWorkspace>,
    autoload: Option<ResMut<Autoload>>,
) {
    if let Some(mut autoload) = autoload {
        if let Some(filename) = autoload.filename.take() {
            let Ok(data) = std::fs::read(&filename) else {
                error!("Failed opening file {}", filename.to_string_lossy());
                exit.send(AppExit);
                return;
            };
            send_load_site_event(&mut load_site, &filename, &data);
        }
        return;
    }

    egui::Window::new("Welcome!")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
        .show(egui_context.ctx_mut(), |ui| {
            ui.heading("Welcome to The RMF Site Editor!");
            ui.add_space(10.);

            ui.horizontal(|ui| {
                if ui.button("View demo map").clicked() {
                    match BuildingMap::from_bytes(&demo_office()) {
                        Ok(building) => match building.to_site() {
                            Ok(site) => {
                                load_site.send(LoadSite {
                                    site,
                                    focus: true,
                                    default_file: None,
                                });
                            }
                            Err(err) => {
                                error!("{err:?}");
                            }
                        },
                        Err(err) => {
                            error!("{:?}", err);
                        }
                    }
                }

                if ui.button("Open a file").clicked() {
                    load_workspace.send(LoadWorkspace::Dialog);
                }
            });

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.add_space(20.);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        if ui.button("Exit").clicked() {
                            exit.send(AppExit);
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
