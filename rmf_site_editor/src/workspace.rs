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

use bevy::{
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use rfd::{AsyncFileDialog};
use futures_lite::future;
use std::path::PathBuf;

use crate::AppState;
use crate::interaction::InteractionState;
use crate::site::{LoadSite};
use crate::workcell::{LoadWorkcell};
use rmf_site_format::{Site, SiteProperties, Workcell};
use rmf_site_format::legacy::building_map::BuildingMap;

/// Used as an event to command that a new workspace should be made the current one
#[derive(Clone, Copy, Debug)]
pub struct ChangeCurrentWorkspace {
    /// What should the current site be
    pub root: Entity,
}

/// Used as an event to command that a new workspace should be created, behavior will depend on
/// what app mode the editor is currently in
pub struct CreateNewWorkspace;

/// Used as an event to command that a workspace should be loaded. This will spawn a file open
/// dialog (in non-wasm) with allowed extensions depending on the app state
// TODO(luca) Encapsulate a list of optional filters, for example to allow users to only load
// workcells or sites
// Dialog will spawn a RFD dialog, Path will open a specific path, the others will parse embedded
// data
pub enum LoadWorkspace {
    Dialog,
    Path(PathBuf),
    Data(WorkspaceData),
}

pub enum WorkspaceData {
    LegacyBuilding(Vec<u8>),
    Site(Vec<u8>),
    Workcell(Vec<u8>),
}

impl WorkspaceData {
    pub fn new(path: &PathBuf, data: Vec<u8>) -> Option<Self> {
        let filename = path.file_name().and_then(|f| f.to_str())?;
        if filename.ends_with(".building.yaml") {
            Some(WorkspaceData::LegacyBuilding(data))
        } else if filename.ends_with("site.ron") {
            Some(WorkspaceData::Site(data))
        } else if filename.ends_with("workcell.json") {
            Some(WorkspaceData::Workcell(data))
        } else {
            println!("Unrecognized file type {:?}", filename);
            None
        }
    }
}

/// Used as a resource that keeps track of the current workspace
#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct CurrentWorkspace {
    pub root: Option<Entity>,
    pub display: bool,
}

struct LoadWorkspaceFile(pub std::path::PathBuf, pub Vec<u8>);

#[derive(Component)]
struct LoadWorkspaceFileTask(pub Task<Option<LoadWorkspaceFile>>);

/// Used to keep track of visibility when switching workspace
#[derive(Debug, Default, Resource)]
pub struct RecallWorkspace(Option<Entity>);

impl CurrentWorkspace {
    pub fn to_site(self, open_sites: &Query<Entity, With<SiteProperties>>) -> Option<Entity> {
        let site_entity = self.root?;
        open_sites.get(site_entity).ok()
    }
}

pub struct WorkspacePlugin;

impl Plugin for WorkspacePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ChangeCurrentWorkspace>()
           .add_event::<CreateNewWorkspace>()
           .add_event::<LoadWorkspace>()
           .init_resource::<CurrentWorkspace>()
           .init_resource::<RecallWorkspace>()
           .add_system(dispatch_new_workspace_events)
           .add_system(workspace_file_load_complete)
           .add_system(sync_workspace_visibility);
        #[cfg(not(target_arch = "wasm32"))]
        app.add_system(dispatch_load_workspace_events);
    }
}

pub fn dispatch_new_workspace_events(
    state: Res<State<AppState>>,
    mut new_workspace: EventReader<CreateNewWorkspace>,
    mut load_site: EventWriter<LoadSite>,
    mut load_workcell: EventWriter<LoadWorkcell>,
) {
    if let Some(_cmd) = new_workspace.iter().last() {
        match state.current() {
            AppState::MainMenu => {
                println!("DEV ERROR: Sent generic change workspace while in main menu");
            },
            AppState::SiteEditor => {
                load_site.send(LoadSite { site: Site::default(), focus: true, default_file: None });
            },
            AppState::WorkcellEditor => {
                load_workcell.send(LoadWorkcell { workcell: Workcell::default(), focus: true, default_file: None });
            },
        }
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub fn dispatch_load_workspace_events(
    mut commands: Commands,
    mut app_state: ResMut<State<AppState>>,
    mut interaction_state: ResMut<State<InteractionState>>,
    mut load_site: EventWriter<LoadSite>,
    mut load_workcell: EventWriter<LoadWorkcell>,
    mut load_workspace: EventReader<LoadWorkspace>,
) {
    if let Some(cmd) = load_workspace.iter().last() {
        match cmd {
            LoadWorkspace::Dialog => {
                let future = AsyncComputeTaskPool::get().spawn(async move {
                    let file = AsyncFileDialog::new().pick_file().await?;
                    let data = file.read().await;

                    // TODO(luca) on wasm there is no file path, only file name, put a config guard to
                    // populate accordingly
                    // Full Wasm file loading support would need adding a crate for channels and detaching the thread
                    Some(LoadWorkspaceFile(
                        file.path().to_path_buf(),
                        data
                    ))
                });
                commands.spawn(LoadWorkspaceFileTask(future));
            },
            LoadWorkspace::Path(path) => {
                if let Some(data) = std::fs::read(&path).ok().and_then(|d| WorkspaceData::new(&path, d)) {
                    handle_workspace_data(Some(path.clone()), &data, &mut app_state, &mut interaction_state, &mut load_site, &mut load_workcell);
                }
            },
            LoadWorkspace::Data(data) => {
                // Do a sync load and state update
                handle_workspace_data(None, &data, &mut app_state, &mut interaction_state, &mut load_site, &mut load_workcell);
            },
        }
    }
}

fn handle_workspace_data(
    file: Option<PathBuf>,
    workspace_data: &WorkspaceData,
    app_state: &mut ResMut<State<AppState>>,
    interaction_state: &mut ResMut<State<InteractionState>>,
    load_site: &mut EventWriter<LoadSite>,
    load_workcell: &mut EventWriter<LoadWorkcell>,
) {
    match workspace_data {
        WorkspaceData::LegacyBuilding(data) => {
            println!("Opening legacy building map file");
            if let Some(site) = BuildingMap::from_bytes(&data).ok().and_then(|b| b.to_site().ok()) {
                // Switch state
                app_state.set(AppState::SiteEditor).ok();
                load_site.send(LoadSite {
                    site,
                    focus: true,
                    default_file: file,
                });
                interaction_state.set(InteractionState::Enable).ok();
            } else {
                // TODO(luca) restore more informative errors
                println!("Failed loading legacy building");
            }
        },
        WorkspaceData::Site(data) => {
            println!("Opening site file");
            if let Ok(site) = Site::from_bytes(&data) {
                // Switch state
                app_state.set(AppState::SiteEditor).ok();
                load_site.send(LoadSite {
                    site,
                    focus: true,
                    default_file: file,
                });
                interaction_state.set(InteractionState::Enable).ok();
            } else {
                println!("Failed loading site");
            }
        },
        WorkspaceData::Workcell(data) => {
            println!("Opening workcell file");
            match Workcell::from_bytes(&data) {
                Ok(workcell) => {
                    // Switch state
                    app_state.set(AppState::WorkcellEditor).ok();
                    load_workcell.send(LoadWorkcell {
                        workcell,
                        focus: true,
                        default_file: file,
                    });
                    interaction_state.set(InteractionState::Enable).ok();
                },
                Err(err) =>  {
                println!("Failed loading workcell {:?}", err);
                }
            }
        },
    }
}

/// Handles the file opening events
fn workspace_file_load_complete(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut LoadWorkspaceFileTask)>,
    mut app_state: ResMut<State<AppState>>,
    mut interaction_state: ResMut<State<InteractionState>>,
    mut load_site: EventWriter<LoadSite>,
    mut load_workcell: EventWriter<LoadWorkcell>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            commands.entity(entity).despawn();
            if let Some(result) = result {
                let LoadWorkspaceFile(file, data) = result;
                if let Some(workspace_data) = WorkspaceData::new(&file, data) {
                    handle_workspace_data(Some(file), &workspace_data, &mut app_state, &mut interaction_state, &mut load_site, &mut load_workcell);
                }
            }
        }
    }
}

pub fn sync_workspace_visibility(
    current_workspace: Res<CurrentWorkspace>,
    mut recall: ResMut<RecallWorkspace>,
    mut visibility: Query<&mut Visibility>,
) {
    if !current_workspace.is_changed() {
        return;
    }

    if recall.0 != current_workspace.root {
        // Set visibility of current to target
        if let Some(current_workspace_entity) = current_workspace.root {
            if let Ok(mut v) = visibility.get_mut(current_workspace_entity) {
                v.is_visible = current_workspace.display;
            }
        }
        // Disable visibility in recall
        if let Some(recall) = recall.0 {
            if let Ok(mut v) = visibility.get_mut(recall) {
                v.is_visible = false;
            }
        }
        recall.0 = current_workspace.root;
    }
}
