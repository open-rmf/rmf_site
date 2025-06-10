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

use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_impulse::*;
use rfd::AsyncFileDialog;
use std::path::PathBuf;

use crate::interaction::InteractionState;
use crate::site::{DefaultFile, LoadSite, SaveSite, ImportNavGraphs};
use crate::AppState;
use rmf_site_format::legacy::building_map::BuildingMap;
use rmf_site_format::{NameOfSite, Site};

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

#[derive(Clone)]
pub enum WorkspaceData {
    LegacyBuilding(Vec<u8>),
    RonSite(Vec<u8>),
    JsonSite(Vec<u8>),
    LoadSite(LoadSite),
}

impl WorkspaceData {
    pub fn new(path: &PathBuf, data: Vec<u8>) -> Option<Self> {
        let filename = path.file_name().and_then(|f| f.to_str())?;
        if filename.ends_with(".building.yaml") {
            Some(WorkspaceData::LegacyBuilding(data))
        } else if filename.ends_with(".ron") {
            Some(WorkspaceData::RonSite(data))
        } else if filename.ends_with(".json") {
            Some(WorkspaceData::JsonSite(data))
        } else {
            error!("Unrecognized file type {:?}", filename);
            None
        }
    }

    pub fn as_site(&self) -> Option<Site> {
        match self {
            Self::LegacyBuilding(data) => {
                match BuildingMap::from_bytes(data) {
                    Ok(building) => {
                        match building.to_site() {
                            Ok(site) => return Some(site),
                            Err(err) => {
                                error!("Failed converting a legacy building into a site: {err}");
                                return None;
                            }
                        }
                    }
                    Err(err) => {
                        error!("Failed parsing legacy building: {err}");
                        return None;
                    }
                }
            }
            Self::RonSite(data) => {
                match Site::from_bytes_ron(data) {
                    Ok(site) => return Some(site),
                    Err(err) => {
                        error!("Failed parsing ron site file: {err}");
                        return None;
                    }
                }
            }
            Self::JsonSite(data) => {
                match Site::from_bytes_json(data) {
                    Ok(site) => return Some(site),
                    Err(err) => {
                        error!("Failed loading json site file: {err}");
                        return None;
                    }
                }
            }
            Self::LoadSite(load) => return Some(load.site.clone()),
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

#[derive(Clone, Default, Debug)]
pub enum ExportFormat {
    #[default]
    Default,
    Sdf,
    NavGraph,
}

/// Used to keep track of visibility when switching workspace
#[derive(Debug, Default, Resource)]
pub struct RecallWorkspace(pub Option<Entity>);

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
            .init_resource::<CurrentWorkspace>()
            .init_resource::<RecallWorkspace>()
            .init_resource::<FileDialogServices>()
            .init_resource::<WorkspaceLoadingServices>()
            .init_resource::<WorkspaceSavingServices>()
            .add_systems(
                Update,
                (dispatch_new_workspace_events, sync_workspace_visibility),
            );
    }
}

pub fn dispatch_new_workspace_events(
    state: Res<State<AppState>>,
    mut new_workspace: EventReader<CreateNewWorkspace>,
    mut load_site: EventWriter<LoadSite>,
) {
    if let Some(_cmd) = new_workspace.read().last() {
        match state.get() {
            AppState::MainMenu => {
                error!("Sent generic new workspace while in main menu");
            }
            AppState::SiteEditor | AppState::SiteDrawingEditor | AppState::SiteVisualizer => {
                load_site.write(LoadSite {
                    site: Site::blank_L1("new".to_owned()),
                    focus: true,
                    default_file: None,
                });
            }
        }
    }
}

/// Service that takes workspace data and loads a site / workcell, as well as transition state.
pub fn process_load_workspace_files(
    In(BlockingService { request, .. }): BlockingServiceInput<LoadWorkspaceFile>,
    mut app_state: ResMut<NextState<AppState>>,
    mut interaction_state: ResMut<NextState<InteractionState>>,
    mut load_site: EventWriter<LoadSite>,
) {
    let LoadWorkspaceFile(default_file, data) = request;
    match data {
        WorkspaceData::LegacyBuilding(data) => {
            info!("Opening legacy building map file");
            match BuildingMap::from_bytes(&data) {
                Ok(building) => {
                    match building.to_site() {
                        Ok(site) => {
                            // Switch state
                            app_state.set(AppState::SiteEditor);
                            load_site.write(LoadSite {
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
                    load_site.write(LoadSite {
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
                    load_site.write(LoadSite {
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
        WorkspaceData::LoadSite(site) => {
            app_state.set(AppState::SiteEditor);
            load_site.write(site);
            interaction_state.set(InteractionState::Enable);
        }
    }
}

/// Filter that can be added to a file dialog request to filter extensions
#[derive(Clone, Debug)]
pub struct FileDialogFilter {
    pub name: String,
    pub extensions: Vec<String>,
}

/// Services that spawn async file dialogs for various purposes, i.e. loading files, saving files /
/// folders.
#[derive(Resource)]
pub struct FileDialogServices {
    /// Open a dialog to pick a file, return its path and data
    pub pick_file_and_load: Service<Vec<FileDialogFilter>, (PathBuf, Vec<u8>)>,
    /// Pick a file to save data into
    pub pick_file_for_saving: Service<Vec<FileDialogFilter>, PathBuf>,
    /// Pick a folder
    pub pick_folder: Service<(), PathBuf>,
}

impl FromWorld for FileDialogServices {
    fn from_world(world: &mut World) -> Self {
        let pick_file_and_load = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_async(|filters: Vec<FileDialogFilter>| async move {
                    let mut dialog = AsyncFileDialog::new();
                    for filter in filters {
                        dialog = dialog.add_filter(filter.name, &filter.extensions);
                    }
                    if let Some(file) = dialog.pick_file().await {
                        let path = file.path();
                        match std::fs::metadata(path) {
                            Ok(meta) => {
                                if meta.is_dir() {
                                    error!("Selected directory when a file is needed: {}", path.as_os_str().to_string_lossy());
                                    return None;
                                }
                            }
                            Err(err) => {
                                error!("Did not select a valid file [{}], error: {err}", path.as_os_str().to_string_lossy());
                                return None;
                            }
                        }

                        let data = file.read().await;
                        #[cfg(not(target_arch = "wasm32"))]
                        let file = file.path().to_path_buf();
                        #[cfg(target_arch = "wasm32")]
                        let file = PathBuf::from(file.file_name());
                        return Some((file, data));
                    }
                    None
                })
                .cancel_on_none()
                .connect(scope.terminate)
        });

        let pick_file_for_saving = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_async(|filters: Vec<FileDialogFilter>| async move {
                    let mut dialog = AsyncFileDialog::new();
                    for filter in filters {
                        dialog = dialog.add_filter(filter.name, &filter.extensions);
                    }
                    let file = dialog.save_file().await?;
                    #[cfg(not(target_arch = "wasm32"))]
                    let file = file.path().to_path_buf();
                    #[cfg(target_arch = "wasm32")]
                    let file = PathBuf::from(file.file_name());
                    Some(file)
                })
                .cancel_on_none()
                .connect(scope.terminate)
        });

        let pick_folder = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_async(|_| async move {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        AsyncFileDialog::new()
                            .pick_folder()
                            .await
                            .map(|f| f.path().into())
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        warn!("Folder dialogs are not implemented in wasm");
                        None
                    }
                })
                .cancel_on_none()
                .connect(scope.terminate)
        });

        Self {
            pick_file_and_load,
            pick_file_for_saving,
            pick_folder,
        }
    }
}

#[derive(Resource)]
/// Services that deal with workspace loading
pub struct WorkspaceLoadingServices {
    /// Service that spawns an open file dialog and loads a site accordingly.
    pub load_workspace_from_dialog: Service<(), ()>,
    /// Service that spawns a save file dialog then creates a site with an empty level.
    pub create_empty_workspace_from_dialog: Service<(), ()>,
    /// Loads the workspace at the requested path
    pub load_workspace_from_path: Service<PathBuf, ()>,
    /// Loads the workspace from the requested data
    pub load_workspace_from_data: Service<WorkspaceData, ()>,
    /// Service that lets the user select a file to import nav graphs from.
    pub import_nav_graphs_from_dialog: Service<(), ()>,
}

impl FromWorld for WorkspaceLoadingServices {
    fn from_world(world: &mut World) -> Self {
        let process_load_files = world.spawn_service(process_load_workspace_files);
        let pick_file = world
            .resource::<FileDialogServices>()
            .pick_file_and_load
            .clone();
        let loading_filters = vec![
            FileDialogFilter {
                name: "Site or Building".into(),
                extensions: vec![
                    "site.ron".into(),
                    "site.json".into(),
                    "building.yaml".into(),
                ],
            },
            FileDialogFilter {
                name: "Structured file".into(),
                extensions: vec![
                    "ron".into(),
                    "json".into(),
                    "yaml".into(),
                ]
            },
            FileDialogFilter {
                name: "All files".into(),
                extensions: vec!["*".into()],
            },
        ];
        // Spawn all the services
        let load_workspace_from_dialog = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_block({
                    let loading_filters = loading_filters.clone();
                    move |_| loading_filters.clone()
                })
                .then(pick_file)
                .map_block(|(path, data)| {
                    let data = WorkspaceData::new(&path, data)?;
                    Some(LoadWorkspaceFile(Some(path), data))
                })
                .cancel_on_none()
                .then(process_load_files)
                .connect(scope.terminate)
        });

        let create_empty_workspace_from_dialog = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_async(|_| async move {
                    #[cfg(not(target_arch = "wasm32"))]
                    {
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
                            return Some(LoadWorkspaceFile(Some(file), data));
                        }
                        None
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        let data =
                            WorkspaceData::LoadSite(LoadSite::blank_L1("blank".to_owned(), None));
                        Some(LoadWorkspaceFile(None, data))
                    }
                })
                .cancel_on_none()
                .then(process_load_files)
                .connect(scope.terminate)
        });

        let load_workspace_from_path = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_block(|path| {
                    let Some(data) = std::fs::read(&path)
                        .ok()
                        .and_then(|data| WorkspaceData::new(&path, data))
                    else {
                        warn!("Unable to read file [{path:?}] so it cannot be loaded");
                        return None;
                    };
                    Some(LoadWorkspaceFile(Some(path.clone()), data))
                })
                .cancel_on_none()
                .then(process_load_files)
                .connect(scope.terminate)
        });

        let load_workspace_from_data = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_block(|data| LoadWorkspaceFile(None, data))
                .then(process_load_files)
                .connect(scope.terminate)
        });

        let request_import_nav_graphs = |
            In(from_site): In<Site>,
            current_site: Res<CurrentWorkspace>,
            mut import_nav_graphs: EventWriter<ImportNavGraphs>,
        | {
            let Some(into_site) = current_site.root else {
                return;
            };
            import_nav_graphs.write(ImportNavGraphs { into_site, from_site });
        };

        let import_nav_graphs_from_dialog = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_block({
                    let loading_filters = loading_filters.clone();
                    move |_| loading_filters.clone()
                })
                .then(pick_file)
                .map_async(|(path, data)| async move {
                    WorkspaceData::new(&path, data)?.as_site()
                })
                .cancel_on_none()
                .then(request_import_nav_graphs.into_blocking_callback())
                .connect(scope.terminate);
        });

        Self {
            load_workspace_from_dialog,
            create_empty_workspace_from_dialog,
            load_workspace_from_path,
            load_workspace_from_data,
            import_nav_graphs_from_dialog,
        }
    }
}

impl<'w, 's> WorkspaceLoader<'w, 's> {
    /// Request to spawn a dialog and load a workspace
    pub fn load_from_dialog(&mut self) {
        self.commands
            .request((), self.workspace_loading.load_workspace_from_dialog)
            .detach();
    }

    /// Request to spawn a dialog to select a file and create a new site with a blank level
    pub fn create_empty_from_dialog(&mut self) {
        self.commands
            .request(
                (),
                self.workspace_loading.create_empty_workspace_from_dialog,
            )
            .detach();
    }

    /// Request to load a workspace from a path
    pub fn load_from_path(&mut self, path: PathBuf) {
        self.commands
            .request(path, self.workspace_loading.load_workspace_from_path)
            .detach();
    }

    /// Request to load a workspace from data
    pub fn load_from_data(&mut self, data: WorkspaceData) {
        self.commands
            .request(data, self.workspace_loading.load_workspace_from_data)
            .detach();
    }

    pub fn import_nav_graphs_from_dialog(&mut self) {
        self.commands
            .request((), self.workspace_loading.import_nav_graphs_from_dialog)
            .detach();
    }
}

/// `SystemParam` used to request for workspace loading operations
#[derive(SystemParam)]
pub struct WorkspaceLoader<'w, 's> {
    workspace_loading: Res<'w, WorkspaceLoadingServices>,
    commands: Commands<'w, 's>,
}

/// Handles the file saving events
fn send_file_save(
    In(BlockingService { request, .. }): BlockingServiceInput<(PathBuf, ExportFormat)>,
    app_state: Res<State<AppState>>,
    mut save_site: EventWriter<SaveSite>,
    current_workspace: Res<CurrentWorkspace>,
) {
    let Some(ws_root) = current_workspace.root else {
        warn!("Failed saving workspace, no current workspace found");
        return;
    };
    match app_state.get() {
        AppState::SiteEditor | AppState::SiteDrawingEditor | AppState::SiteVisualizer => {
            save_site.write(SaveSite {
                site: ws_root,
                to_location: request.0,
                format: request.1,
            });
        }
        AppState::MainMenu => { /* Noop */ }
    }
}

#[derive(Resource)]
/// Services that deal with workspace loading
pub struct WorkspaceSavingServices {
    /// Service that spawns a save file dialog and saves the current site accordingly.
    pub save_workspace_to_dialog: Service<(), ()>,
    /// Saves the current workspace at the requested path.
    pub save_workspace_to_path: Service<PathBuf, ()>,
    /// Saves the current workspace in the current default file.
    pub save_workspace_to_default_file: Service<(), ()>,
    /// Opens a dialog to pick a folder and exports the requested workspace as an SDF.
    pub export_sdf_to_dialog: Service<(), ()>,
    /// Exports the requested workspace as an SDF in the requested path.
    pub export_sdf_to_path: Service<PathBuf, ()>,
    /// Opens a dialog to pick a folder and exports the nav graphs from the requested site.
    pub export_nav_graphs_to_dialog: Service<(), ()>,
    /// Exports the nav graphs from the requested site to the requested path.
    pub export_nav_graphs_to_path: Service<PathBuf, ()>,
}

impl FromWorld for WorkspaceSavingServices {
    fn from_world(world: &mut World) -> Self {
        let send_file_save = world.spawn_service(send_file_save);
        let get_default_file = |In(()): In<_>,
                                current_workspace: Res<CurrentWorkspace>,
                                default_files: Query<&DefaultFile>|
         -> Option<PathBuf> {
            let ws_root = current_workspace.root?;
            default_files.get(ws_root).ok().map(|f| f.0.clone())
        };
        let get_default_file = get_default_file.into_blocking_callback();
        let pick_file = world
            .resource::<FileDialogServices>()
            .pick_file_for_saving
            .clone();
        let pick_folder = world.resource::<FileDialogServices>().pick_folder.clone();
        let saving_filters = vec![
            FileDialogFilter {
                name: "Site".into(),
                extensions: vec!["site.json".into(), "site.ron".into()],
            },
            FileDialogFilter {
                name: "All Files".into(),
                extensions: vec!["*".into()],
            },
        ];

        // Spawn all the services
        let save_workspace_to_dialog = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_block(move |_| saving_filters.clone())
                .then(pick_file)
                .map_block(|path| (path, ExportFormat::default()))
                .then(send_file_save)
                .connect(scope.terminate)
        });
        let save_workspace_to_path = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_block(|path| (path, ExportFormat::default()))
                .then(send_file_save)
                .connect(scope.terminate)
        });
        let save_workspace_to_default_file = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .then(get_default_file)
                .branch_for_none(|chain: Chain<()>| {
                    chain
                        .then(save_workspace_to_dialog)
                        .connect(scope.terminate)
                })
                .then(save_workspace_to_path)
                .connect(scope.terminate)
        });
        let export_sdf_to_dialog = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .then(pick_folder)
                .map_block(|path| (path, ExportFormat::Sdf))
                .then(send_file_save)
                .connect(scope.terminate)
        });
        let export_sdf_to_path = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_block(|path| (path, ExportFormat::Sdf))
                .then(send_file_save)
                .connect(scope.terminate)
        });

        let export_nav_graphs_to_dialog = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .then(pick_folder)
                .map_block(|path| (dbg!(path), ExportFormat::NavGraph))
                .then(send_file_save)
                .connect(scope.terminate)
        });

        let export_nav_graphs_to_path = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_block(|path| (path, ExportFormat::NavGraph))
                .then(send_file_save)
                .connect(scope.terminate)
        });

        Self {
            save_workspace_to_dialog,
            save_workspace_to_path,
            save_workspace_to_default_file,
            export_sdf_to_dialog,
            export_sdf_to_path,
            export_nav_graphs_to_dialog,
            export_nav_graphs_to_path,
        }
    }
}

// TODO(luca) implement saving in wasm, it's supported since rfd version 0.12, however it requires
// calling .write on the `FileHandle` object returned by the AsyncFileDialog. Such FileHandle is
// not Send in wasm so it can't be sent to another thread through an event. We would need to
// refactor saving to be fully done in the async task rather than send an event to have wasm saving.
impl<'w, 's> WorkspaceSaver<'w, 's> {
    /// Request to spawn a dialog and save the workspace
    pub fn save_to_dialog(&mut self) {
        self.commands
            .request((), self.workspace_saving.save_workspace_to_dialog)
            .detach();
    }

    /// Request to save the workspace to the default file (or a dialog if no default file is
    /// available).
    pub fn save_to_default_file(&mut self) {
        self.commands
            .request((), self.workspace_saving.save_workspace_to_default_file)
            .detach();
    }

    /// Request to save the workspace to the requested path
    pub fn save_to_path(&mut self, path: PathBuf) {
        self.commands
            .request(path, self.workspace_saving.save_workspace_to_path)
            .detach();
    }

    /// Request to export the workspace as a sdf to a folder selected from a dialog
    pub fn export_sdf_to_dialog(&mut self) {
        self.commands
            .request((), self.workspace_saving.export_sdf_to_dialog)
            .detach();
    }

    /// Request to export the workspace as a sdf to provided folder
    pub fn export_sdf_to_path(&mut self, path: PathBuf) {
        self.commands
            .request(path, self.workspace_saving.export_sdf_to_path)
            .detach();
    }

    /// Request to export the current nav graphs as a yaml to a folder selected from a dialog
    pub fn export_nav_graphs_to_dialog(&mut self) {
        self.commands
            .request((), self.workspace_saving.export_nav_graphs_to_dialog)
            .detach();
    }
}

/// `SystemParam` used to request for workspace loading operations
#[derive(SystemParam)]
pub struct WorkspaceSaver<'w, 's> {
    workspace_saving: Res<'w, WorkspaceSavingServices>,
    commands: Commands<'w, 's>,
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
