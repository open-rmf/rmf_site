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

use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use rfd::AsyncFileDialog;
use std::path::PathBuf;

use crate::interaction::InteractionState;
use crate::site::{DefaultFile, LoadSite, SaveSite};
use crate::workcell::{LoadWorkcell, SaveWorkcell};
use crate::AppState;
use rmf_site_format::legacy::building_map::BuildingMap;
use rmf_site_format::{NameOfSite, Site, Workcell};

use crossbeam_channel::{Receiver, Sender};

/// Used as an event to command that a new workspace should be made the current one
#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentWorkspace {
    /// What should the current site be
    pub root: Entity,
}

/// Used as an event to command that a new workspace should be created, behavior will depend on
/// what app mode the editor is currently in
#[derive(Event)]
pub struct CreateNewWorkspace;

/// Apply this component to all workspace types
#[derive(Component)]
pub struct WorkspaceMarker;

/// Used as an event to command that a workspace should be loaded. This will spawn a file open
/// dialog (in non-wasm) with allowed extensions depending on the app state
// TODO(luca) Encapsulate a list of optional filters, for example to allow users to only load
// workcells or sites
// Dialog will spawn a RFD dialog, Path will open a specific path, the others will parse embedded
// data
#[derive(Event)]
pub enum LoadWorkspace {
    Dialog,
    BlankFromDialog,
    Path(PathBuf),
    Data(WorkspaceData),
}

#[derive(Clone)]
pub enum WorkspaceData {
    LegacyBuilding(Vec<u8>),
    RonSite(Vec<u8>),
    JsonSite(Vec<u8>),
    Workcell(Vec<u8>),
    WorkcellUrdf(Vec<u8>),
    LoadSite(LoadSite),
}

impl WorkspaceData {
    pub fn new(path: &PathBuf, data: Vec<u8>) -> Option<Self> {
        let filename = path.file_name().and_then(|f| f.to_str())?;
        if filename.ends_with(".building.yaml") {
            Some(WorkspaceData::LegacyBuilding(data))
        } else if filename.ends_with("site.ron") {
            Some(WorkspaceData::RonSite(data))
        } else if filename.ends_with("site.json") {
            Some(WorkspaceData::JsonSite(data))
        } else if filename.ends_with("workcell.json") {
            Some(WorkspaceData::Workcell(data))
        } else if filename.ends_with("urdf") {
            Some(WorkspaceData::WorkcellUrdf(data))
        } else {
            error!("Unrecognized file type {:?}", filename);
            None
        }
    }
}

/// Used as a resource that keeps track of the current workspace
// TODO(@mxgrey): Consider a workspace stack, e.g. so users can temporarily edit
// a workcell inside of a site and then revert back into the site.
#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct CurrentWorkspace {
    pub root: Option<Entity>,
    pub display: bool,
}

pub struct LoadWorkspaceFile(pub Option<PathBuf>, pub WorkspaceData);

/// Using channels instead of events to allow usage in wasm since, unlike event writers, they can
/// be cloned and moved into async functions therefore don't have lifetime issues
#[derive(Debug, Resource)]
pub struct LoadWorkspaceChannels {
    pub sender: Sender<LoadWorkspaceFile>,
    pub receiver: Receiver<LoadWorkspaceFile>,
}

impl Default for LoadWorkspaceChannels {
    fn default() -> Self {
        let (sender, receiver) = crossbeam_channel::unbounded();
        Self { sender, receiver }
    }
}

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

    pub fn to_sdf(mut self) -> Self {
        self.format = ExportFormat::Sdf;
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
    Sdf,
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

/// Used to keep track of visibility when switching workspace
#[derive(Debug, Default, Resource)]
pub struct RecallWorkspace(Option<Entity>);

impl CurrentWorkspace {
    pub fn to_site(self, open_sites: &Query<Entity, With<NameOfSite>>) -> Option<Entity> {
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
            .add_event::<SaveWorkspace>()
            .init_resource::<CurrentWorkspace>()
            .init_resource::<RecallWorkspace>()
            .init_resource::<SaveWorkspaceChannels>()
            .init_resource::<LoadWorkspaceChannels>()
            .add_systems(
                Update,
                (
                    dispatch_new_workspace_events,
                    workspace_file_load_complete,
                    sync_workspace_visibility,
                    dispatch_load_workspace_events,
                    workspace_file_save_complete,
                ),
            );
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, dispatch_save_workspace_events);
    }
}

pub fn dispatch_new_workspace_events(
    state: Res<State<AppState>>,
    mut new_workspace: EventReader<CreateNewWorkspace>,
    mut load_site: EventWriter<LoadSite>,
    mut load_workcell: EventWriter<LoadWorkcell>,
) {
    if let Some(_cmd) = new_workspace.read().last() {
        match state.get() {
            AppState::MainMenu => {
                error!("Sent generic new workspace while in main menu");
            }
            AppState::SiteEditor | AppState::SiteDrawingEditor | AppState::SiteVisualizer => {
                load_site.send(LoadSite {
                    site: Site::blank_L1("new".to_owned()),
                    focus: true,
                    default_file: None,
                });
            }
            AppState::WorkcellEditor => {
                load_workcell.send(LoadWorkcell {
                    workcell: Workcell::default(),
                    focus: true,
                    default_file: None,
                });
            }
        }
    }
}

pub fn dispatch_load_workspace_events(
    load_channels: Res<LoadWorkspaceChannels>,
    mut load_workspace: EventReader<LoadWorkspace>,
) {
    if let Some(cmd) = load_workspace.read().last() {
        match cmd {
            LoadWorkspace::Dialog => {
                let sender = load_channels.sender.clone();
                AsyncComputeTaskPool::get()
                    .spawn(async move {
                        if let Some(file) = AsyncFileDialog::new().pick_file().await {
                            let data = file.read().await;
                            #[cfg(not(target_arch = "wasm32"))]
                            let file = file.path().to_path_buf();
                            #[cfg(target_arch = "wasm32")]
                            let file = PathBuf::from(file.file_name());
                            if let Some(data) = WorkspaceData::new(&file, data) {
                                sender
                                    .send(LoadWorkspaceFile(Some(file), data))
                                    .expect("Failed sending file event");
                            }
                        }
                    })
                    .detach();
            }
            LoadWorkspace::BlankFromDialog => {
                let sender = load_channels.sender.clone();
                #[cfg(not(target_arch = "wasm32"))]
                {
                    AsyncComputeTaskPool::get()
                        .spawn(async move {
                            if let Some(file) = AsyncFileDialog::new().save_file().await {
                                let file = file.path().to_path_buf();
                                let name = file
                                    .file_stem()
                                    .map(|s| s.to_str().map(|s| s.to_owned()))
                                    .flatten()
                                    .unwrap_or_else(|| "blank".to_owned());
                                let data = WorkspaceData::LoadSite(LoadSite::blank_L1(
                                    name,
                                    Some(file.clone()),
                                ));
                                let _ = sender.send(LoadWorkspaceFile(Some(file), data));
                            }
                        })
                        .detach();
                }
                #[cfg(target_arch = "wasm32")]
                {
                    let data =
                        WorkspaceData::LoadSite(LoadSite::blank_L1("blank".to_owned(), None));
                    sender.send(LoadWorkspaceFile(None, data));
                }
            }
            LoadWorkspace::Path(path) => {
                if let Ok(data) = std::fs::read(&path) {
                    if let Some(data) = WorkspaceData::new(path, data) {
                        load_channels
                            .sender
                            .send(LoadWorkspaceFile(Some(path.clone()), data))
                            .expect("Failed sending load event");
                    }
                } else {
                    warn!("Unable to read file [{path:?}] so it cannot be loaded");
                }
            }
            LoadWorkspace::Data(data) => {
                load_channels
                    .sender
                    .send(LoadWorkspaceFile(None, data.clone()))
                    .expect("Failed sending load event");
            }
        }
    }
}

/// Handles the file opening events
fn workspace_file_load_complete(
    mut app_state: ResMut<NextState<AppState>>,
    mut interaction_state: ResMut<NextState<InteractionState>>,
    mut load_site: EventWriter<LoadSite>,
    mut load_workcell: EventWriter<LoadWorkcell>,
    load_channels: Res<LoadWorkspaceChannels>,
) {
    if let Ok(result) = load_channels.receiver.try_recv() {
        let LoadWorkspaceFile(default_file, data) = result;
        match data {
            WorkspaceData::LegacyBuilding(data) => {
                info!("Opening legacy building map file");
                match BuildingMap::from_bytes(&data) {
                    Ok(building) => {
                        match building.to_site() {
                            Ok(site) => {
                                // Switch state
                                app_state.set(AppState::SiteEditor);
                                load_site.send(LoadSite {
                                    site,
                                    focus: true,
                                    default_file,
                                });
                                interaction_state.set(InteractionState::Enable);
                            }
                            Err(err) => {
                                error!("Failed converting to site {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        error!("Failed loading legacy building {:?}", err);
                    }
                }
            }
            WorkspaceData::RonSite(data) => {
                info!("Opening site file");
                match Site::from_bytes_ron(&data) {
                    Ok(site) => {
                        // Switch state
                        app_state.set(AppState::SiteEditor);
                        load_site.send(LoadSite {
                            site,
                            focus: true,
                            default_file,
                        });
                        interaction_state.set(InteractionState::Enable);
                    }
                    Err(err) => {
                        error!("Failed loading site {:?}", err);
                    }
                }
            }
            WorkspaceData::JsonSite(data) => {
                info!("Opening site file");
                match Site::from_bytes_json(&data) {
                    Ok(site) => {
                        // Switch state
                        app_state.set(AppState::SiteEditor);
                        load_site.send(LoadSite {
                            site,
                            focus: true,
                            default_file,
                        });
                        interaction_state.set(InteractionState::Enable);
                    }
                    Err(err) => {
                        error!("Failed loading site {:?}", err);
                    }
                }
            }
            WorkspaceData::Workcell(data) => {
                info!("Opening workcell file");
                match Workcell::from_bytes(&data) {
                    Ok(workcell) => {
                        // Switch state
                        app_state.set(AppState::WorkcellEditor);
                        load_workcell.send(LoadWorkcell {
                            workcell,
                            focus: true,
                            default_file,
                        });
                        interaction_state.set(InteractionState::Enable);
                    }
                    Err(err) => {
                        error!("Failed loading workcell {:?}", err);
                    }
                }
            }
            WorkspaceData::WorkcellUrdf(data) => {
                info!("Importing urdf workcell");
                let Ok(utf) = std::str::from_utf8(&data) else {
                    error!("Failed converting urdf bytes to string");
                    return;
                };
                match urdf_rs::read_from_string(utf) {
                    Ok(urdf) => {
                        // TODO(luca) make this function return a result and this a match statement
                        match Workcell::from_urdf(&urdf) {
                            Ok(workcell) => {
                                // Switch state
                                app_state.set(AppState::WorkcellEditor);
                                load_workcell.send(LoadWorkcell {
                                    workcell,
                                    focus: true,
                                    default_file,
                                });
                                interaction_state.set(InteractionState::Enable);
                            }
                            Err(err) => {
                                error!("Failed converting urdf to workcell {:?}", err);
                            }
                        }
                    }
                    Err(err) => {
                        error!("Failed loading urdf workcell {:?}", err);
                    }
                }
            }
            WorkspaceData::LoadSite(site) => {
                app_state.set(AppState::SiteEditor);
                load_site.send(site);
                interaction_state.set(InteractionState::Enable);
            }
        }
    }
}

// TODO(luca) implement this in wasm, it's supported since rfd version 0.12, however it requires
// calling .write on the `FileHandle` object returned by the AsyncFileDialog. Such FileHandle is
// not Send in wasm so it can't be sent to another thread through an event. We would need to
// refactor saving to be fully done in the async task rather than send an event to have wasm saving.
#[cfg(not(target_arch = "wasm32"))]
fn dispatch_save_workspace_events(
    mut save_events: EventReader<SaveWorkspace>,
    save_channels: Res<SaveWorkspaceChannels>,
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
    for event in save_events.read() {
        if let Some(ws_root) = workspace.root {
            match &event.destination {
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
            }
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
    save_channels: Res<SaveWorkspaceChannels>,
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
                    format: result.format,
                });
            }
            AppState::MainMenu => { /* Noop */ }
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
                *v = if current_workspace.display {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
        // Disable visibility in recall
        if let Some(recall) = recall.0 {
            if let Ok(mut v) = visibility.get_mut(recall) {
                *v = Visibility::Hidden;
            }
        }
        recall.0 = current_workspace.root;
    }
}
