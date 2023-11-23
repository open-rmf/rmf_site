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
use bevy::{prelude::*, tasks::AsyncComputeTaskPool};

use crossbeam_channel::{Receiver, Sender};
use rfd::AsyncFileDialog;

use std::path::PathBuf;

#[derive(Event)]
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

/// Event used in channels to communicate the file handle that was chosen by the user.
#[derive(Debug)]
pub struct SaveWorkspaceFile {
    path: PathBuf,
    format: ExportFormat,
    root: Entity,
}

/// Use a channel since file dialogs are async and channel senders can be cloned and passed into an
/// async block.
#[derive(Debug, Resource)]
pub struct SaveWorkspaceChannels {
    pub sender: Sender<SaveWorkspaceFile>,
    pub receiver: Receiver<SaveWorkspaceFile>,
}

impl Default for SaveWorkspaceChannels {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }
}

pub struct SavePlugin;

impl Plugin for SavePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SaveWorkspace>()
            .init_resource::<SaveWorkspaceChannels>()
            .add_systems(Update, workspace_file_save_complete);
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, dispatch_save_workspace_events);
    }
}

// TODO(luca) implement this in wasm, it's supported since rfd version 0.12, however it requires
// calling .write on the `FileHandle` object returned by the AsyncFileDialog. Such FileHandle is
// not Send in wasm so it can't be sent to another thread through an event. We would need to
// refactor saving to be fully done in the async task rather than send an event to have wasm saving.
#[cfg(not(target_arch = "wasm32"))]
fn dispatch_save_workspace_events(
    mut save_events: EventReader<SaveWorkspace>,
    mut save_channels: ResMut<SaveWorkspaceChannels>,
    workspace: Res<CurrentWorkspace>,
    default_files: Query<&DefaultFile>,
) {
    let spawn_dialog = |format: &ExportFormat, root| {
        let sender = save_channels.sender.clone();
        let format = format.clone();
        AsyncComputeTaskPool::get()
            .spawn(async move {
                if let Some(file) = AsyncFileDialog::new().save_file().await {
                    let path = file.path().to_path_buf();
                    sender
                        .send(SaveWorkspaceFile { path, format, root })
                        .expect("Failed sending save event");
                }
            })
            .detach();
    };
    for event in save_events.iter() {
        if let Some(ws_root) = workspace.root {
            let path = match &event.destination {
                SaveWorkspaceDestination::DefaultFile => {
                    if let Some(file) = default_files.get(ws_root).ok().map(|f| f.0.clone()) {
                        save_channels
                            .sender
                            .send(SaveWorkspaceFile {
                                path: file,
                                format: event.format.clone(),
                                root: ws_root,
                            })
                            .expect("Failed sending save request");
                    } else {
                        spawn_dialog(&event.format, ws_root);
                    }
                }
                SaveWorkspaceDestination::Dialog => spawn_dialog(&event.format, ws_root),
                SaveWorkspaceDestination::Path(path) => {
                    save_channels
                        .sender
                        .send(SaveWorkspaceFile {
                            path: path.clone(),
                            format: event.format.clone(),
                            root: ws_root,
                        })
                        .expect("Failed sending save request");
                }
            };
        } else {
            warn!("Unable to save, no workspace loaded");
            return;
        }
    }
}

/// Handles the file saving events
fn workspace_file_save_complete(
    app_state: Res<State<AppState>>,
    mut save_site: EventWriter<SaveSite>,
    mut save_workcell: EventWriter<SaveWorkcell>,
    mut save_channels: ResMut<SaveWorkspaceChannels>,
) {
    if let Ok(result) = save_channels.receiver.try_recv() {
        match app_state.get() {
            AppState::WorkcellEditor => {
                save_workcell.send(SaveWorkcell {
                    root: result.root,
                    to_file: result.path,
                    format: result.format,
                });
            }
            AppState::SiteEditor | AppState::SiteDrawingEditor | AppState::SiteVisualizer => {
                save_site.send(SaveSite {
                    site: result.root,
                    to_file: result.path,
                });
            }
            AppState::MainMenu => { /* Noop */ }
        }
    }
}
