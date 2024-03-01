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
    log, site::LoadSite, AppEvents, AppState, CreateNewWorkspace, CurrentWorkspace, LoadWorkspace,
    SaveWorkspace, SaveWorkspaceChannels, WorkspaceData,
};

use bevy::{app::AppExit, prelude::*, tasks::Task};
use bevy_egui::{egui, EguiContexts};
use rmf_site_format::{Level, Site};
use std::{collections::BTreeMap, path::PathBuf};

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
        // return if autoload is empty
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
    mut _load_workspace: EventWriter<LoadWorkspace>,
    autoload: Option<ResMut<WebAutoLoad>>,
) {
    #[cfg(target_arch = "wasm32")]
    {
        // if autoload is empty trigger event
        if autoload.is_none() {
            let mut levels = BTreeMap::new();
            levels.insert(0, Level::default());

            // create new site and convert to bytes
            let site = Site {
                levels,
                ..default()
            };
            // convert site to json using serde
            // let site_json = serde_json::to_string(&site).unwrap();
            // log(&format!("Main Menu - Creating new site: {}", site_json));

            // convert site to bytes
            let site_bytes = ron::to_string(&site).unwrap().as_bytes().to_vec();

            _load_workspace.send(LoadWorkspace::Data(WorkspaceData::Site(site_bytes)));
        }
    }

    egui::Window::new("Welcome to RCC Traffic Editor!")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
        .show(egui_context.ctx_mut(), |ui| {
            ui.heading("Loading RCC RMF Site Editor...");
        });
}

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(Update, (egui_ui).run_if(in_state(AppState::MainMenu)));
    }
}
