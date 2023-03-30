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

use bevy::prelude::*;
use crate::{AppState, CurrentWorkspace};
use crate::site::{DefaultFile, SaveSite};
use crate::workcell::SaveWorkcell;

use std::path::PathBuf;

#[derive(Default)]
pub struct SaveWorkspace {
    /// If specified workspace will be saved to requested file, otherwise the default file
    pub to_file: Option<PathBuf>,
    /// If specified the workspace will be exported to a specific format
    pub format: ExportFormat,
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
        app.add_event::<SaveWorkspace>()
           .add_system(dispatch_save_events);
    }
}

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
            if let Some(file) = event.to_file.clone().or(default_files.get(ws_root).ok().map(|f| f.0.clone())) {
                match app_state.current() {
                    AppState::WorkcellEditor => {
                        save_workcell.send(SaveWorkcell {
                            root: ws_root,
                            to_file: file,
                            format: event.format.clone(),
                        });
                    }
                    // TODO(luca) migrate site/save as well to non optional path?
                    AppState::SiteEditor => {
                        save_site.send(SaveSite {
                            site: ws_root,
                            to_file: Some(file),
                        });
                    }
                    AppState::MainMenu => { /* Noop */ }
                }
            } else {
                println!("No file specified, use File -> Save As");
                continue;
            }
        } else {
            println!("Unable to save, no workspace loaded");
            return;
        }
    }
}
