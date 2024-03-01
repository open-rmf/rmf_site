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
use crate::{
    log, AppEvents, AppState, CreateNewWorkspace, CurrentWorkspace, LoadWorkspace, SaveWorkspace,
    SaveWorkspaceChannels, WorkspaceData,
};

use bevy::{app::AppExit, prelude::*, tasks::Task};
use bevy_egui::{egui, EguiContexts};
use std::path::PathBuf;

#[derive(Resource)]
pub struct Autoload {
    pub filename: Option<PathBuf>,
    pub import: Option<PathBuf>,
    pub importing: Option<Task<Option<(Entity, rmf_site_format::Site)>>>,
}

impl Autoload {
    pub fn file(filename: PathBuf, import: Option<PathBuf>) -> Self {
        Autoload {
            filename: Some(filename),
            import,
            importing: None,
        }
    }
}

#[derive(Resource)]
pub struct SaveToWeb {
    pub url: String,
}

#[derive(Resource)]
pub struct WebAutoLoad {
    pub building_data: Option<Vec<u8>>,
    pub file_type: String,
}

impl WebAutoLoad {
    pub fn file(url: Vec<u8>, file_type: String) -> Self {
        WebAutoLoad {
            building_data: Some(url),
            file_type: file_type,
        }
    }
}

#[derive(Resource)]
pub struct UploadData {
    pub building_id: Option<String>,
}

impl UploadData {
    pub fn new(building_id: String) -> Self {
        UploadData {
            building_id: Some(building_id),
        }
    }
}

fn autoload_from_web(
    // access resource
    mut _load_workspace: EventWriter<LoadWorkspace>,
    autoload: Option<ResMut<WebAutoLoad>>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        log(&format!(
            "Main Menu - Trying to load map from building data"
        ));

        if let Some(autoload) = autoload {
            if let Some(building_data) = autoload.building_data.clone() {
                #[cfg(target_arch = "wasm32")]
                log(&format!(
                    "Main Menu - Loading map from building data with format {}",
                    autoload.file_type
                ));

                match autoload.file_type.as_str() {
                    "building.yaml" => {
                        _load_workspace.send(LoadWorkspace::Data(WorkspaceData::LegacyBuilding(
                            building_data,
                        )));
                    }
                    "site.ron" => {
                        _load_workspace
                            .send(LoadWorkspace::Data(WorkspaceData::Site(building_data)));
                    }
                    _ => {
                        log(&format!(
                            "Main Menu - Unsupported file type: {}",
                            autoload.file_type
                        ));
                    }
                }
            }
        }
    }
}

fn egui_ui(
    mut egui_context: EguiContexts,
    mut events: AppEvents,
    mut _exit: EventWriter<AppExit>,
    mut _load_workspace: EventWriter<LoadWorkspace>,
    mut _app_state: ResMut<State<AppState>>,
    autoload: Option<ResMut<Autoload>>,
) {
    #[cfg(not(target_arch = "wasm32"))]
    {
        if let Some(mut autoload) = autoload {
            {
                if let Some(filename) = autoload.filename.clone() {
                    _load_workspace.send(LoadWorkspace::Path(filename));
                }
                autoload.filename = None;
            }
            return;
        }
    }

    // if auto load is empty, trigger new workspace
    if let Some(autoload) = autoload {
        if autoload.filename.is_none() {
            let _ = &events.file_events.new_workspace.send(CreateNewWorkspace);
        }
    }

    egui::Window::new("Welcome to RCC Traffic Editor!")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
        .show(egui_context.ctx_mut(), |ui| {
            ui.heading("Loading RCC RMF Site Editor...");
            // ui.add_space(10.);

            // ui.horizontal(|ui| {
            //     if ui.button("View demo map").clicked() {
            //         _load_workspace.send(LoadWorkspace::Data(WorkspaceData::LegacyBuilding(
            //             demo_office(),
            //         )));
            //     }

            //     // if ui.button("Open a file").clicked() {
            //     //     _load_workspace.send(LoadWorkspace::Dialog);
            //     // }

            //     // TODO(@mxgrey): Bring this back when we have finished developing
            //     // the key features for workcell editing.
            //     // if ui.button("Workcell Editor").clicked() {
            //     //     _load_workspace.send(LoadWorkspace::Data(WorkspaceData::Workcell(
            //     //         demo_workcell(),
            //     //     )));
            //     // }

            //     // TODO(@mxgrey): Bring this back when we have time to fix the
            //     // warehouse generator.
            //     // if ui.button("Warehouse generator").clicked() {
            //     //     info!("Entering warehouse generator");
            //     //     _app_state.overwrite_set(AppState::WarehouseGenerator).unwrap();
            //     // }
            // });

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
        app.add_systems(
            Update,
            (egui_ui, autoload_from_web).run_if(in_state(AppState::MainMenu)),
        );
    }
}
