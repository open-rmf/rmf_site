/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use crate::site::{DefaultFile, SaveSite};
use crate::workcell::SaveWorkcell;
use crate::{AppState, CurrentWorkspace};
use bevy::prelude::*;

#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

use std::path::PathBuf;

pub struct SaveWorkspace {
    /// If specified workspace will be saved to requested file, otherwise the default file
    pub destination: SaveWorkspaceDestination,
    /// If specified the workspace will be exported to a specific format
    pub format: ExportFormat,
}

impl SaveWorkspace {
    pub fn new() -> Self {
        Self {
            destination: SaveWorkspaceDestination::default(),
            format: ExportFormat::default(),
        }
    }

    pub fn to_default_file(mut self) -> Self {
        self.destination = SaveWorkspaceDestination::DefaultFile;
        self
    }

    pub fn to_dialog(mut self) -> Self {
        self.destination = SaveWorkspaceDestination::Dialog;
        self
    }

    pub fn to_path(mut self, path: &PathBuf) -> Self {
        self.destination = SaveWorkspaceDestination::Path(path.clone());
        self
    }

    pub fn to_urdf(mut self) -> Self {
        self.format = ExportFormat::Urdf;
        self
    }
}

#[derive(Default, Debug, Clone)]
pub enum SaveWorkspaceDestination {
    #[default]
    DefaultFile,
    Dialog,
    Path(PathBuf),
}

#[derive(Clone, Default, Debug)]
pub enum ExportFormat {
    #[default]
    Default,
    Urdf,
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SaveWorkspace>();
        #[cfg(not(target_arch = "wasm32"))]
        app.add_system(dispatch_save_events);
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn dispatch_save_events(
    mut save_events: EventReader<SaveWorkspace>,
    mut save_site: EventWriter<SaveSite>,
    mut save_workcell: EventWriter<SaveWorkcell>,
    app_state: Res<State<AppState>>,
    workspace: Res<CurrentWorkspace>,
    default_files: Query<&DefaultFile>,
) {
    for event in save_events.iter() {
        if let Some(ws_root) = workspace.root {
            let path = match &event.destination {
                SaveWorkspaceDestination::DefaultFile => {
                    if let Some(file) = default_files.get(ws_root).ok().map(|f| f.0.clone()) {
                        file
                    } else {
                        let Some(file) = FileDialog::new().save_file() else {
                            continue;
                        };
                        file
                    }
                }
                SaveWorkspaceDestination::Dialog => {
                    // TODO(luca) async impl?
                    let Some(file) = FileDialog::new().save_file() else {
                        continue;
                    };
                    file
                }
                SaveWorkspaceDestination::Path(path) => path.clone(),
            };
            match app_state.current() {
                AppState::WorkcellEditor => {
                    save_workcell.send(SaveWorkcell {
                        root: ws_root,
                        to_file: path,
                        format: event.format.clone(),
                    });
                }
                AppState::SiteEditor => {
                    save_site.send(SaveSite {
                        site: ws_root,
                        to_file: path,
                    });
                }
                AppState::MainMenu => { /* Noop */ }
            }
        } else {
            warn!("Unable to save, no workspace loaded");
            return;
        }
    }
}
