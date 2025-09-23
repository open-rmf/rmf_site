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
use std::{future::Future, path::PathBuf};

use crate::interaction::InteractionState;
use crate::site::{DefaultFile, ImportNavGraphs, LoadSite, LoadSiteResult, SaveSite};
use crate::AppState;
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

/// Used as a resource that keeps track of the current workspace
// TODO(@mxgrey): Consider a workspace stack, e.g. so users can temporarily edit
// a workcell inside of a site and then revert back into the site.
#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct CurrentWorkspace {
    pub root: Option<Entity>,
    pub display: bool,
}

#[derive(Clone, Default, Debug)]
pub enum ExportFormat {
    #[default]
    Default,
    Sdf,
    NavGraph,
    OccupancyGrid,
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
            .init_resource::<SiteLoadingServices>()
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
pub fn send_load_workspace_files(
    In(BlockingService { mut request, .. }): BlockingServiceInput<LoadSite>,
    mut app_state: ResMut<NextState<AppState>>,
    mut interaction_state: ResMut<NextState<InteractionState>>,
    mut load_site: EventWriter<LoadSite>,
) {
    app_state.set(AppState::SiteEditor);
    interaction_state.set(InteractionState::Enable);

    request.focus = true;
    load_site.write(request);
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
    pub pick_file_and_read: Service<Vec<FileDialogFilter>, (PathBuf, Vec<u8>)>,
    /// Pick a file to save data into
    pub pick_file_for_saving: Service<Vec<FileDialogFilter>, PathBuf>,
    /// Pick a folder
    pub pick_folder: Service<(), PathBuf>,
}

impl FromWorld for FileDialogServices {
    fn from_world(world: &mut World) -> Self {
        let pick_file_and_read = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_async(|filters: Vec<FileDialogFilter>| async move {
                    let mut dialog = AsyncFileDialog::new();
                    for filter in filters {
                        dialog = dialog.add_filter(filter.name, &filter.extensions);
                    }
                    if let Some(file) = dialog.pick_file().await {
                        #[cfg(not(target_arch = "wasm32"))]
                        {
                            // This safety check is disabled for wasm32 because
                            // the .path() method is not available for wasm32.
                            // We should try to find a better way to identify
                            // when a directory is chosen instead of a file,
                            // because otherwise a panic will occur inside one
                            // of our dependencies when the user makes that kind
                            // of error.
                            //
                            // We may even consider using a different dialog
                            // library since these limitations make our
                            // implementation awfully fragile.
                            let path = file.path();
                            match std::fs::metadata(path) {
                                Ok(meta) => {
                                    if meta.is_dir() {
                                        error!(
                                            "Selected directory when a file is needed: {}",
                                            path.as_os_str().to_string_lossy()
                                        );
                                        return None;
                                    }
                                }
                                Err(err) => {
                                    error!(
                                        "Did not select a valid file [{}], error: {err}",
                                        path.as_os_str().to_string_lossy()
                                    );
                                    return None;
                                }
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
            pick_file_and_read,
            pick_file_for_saving,
            pick_folder,
        }
    }
}

#[derive(Resource)]
/// Services that deal with workspace loading
pub struct SiteLoadingServices {
    /// Service that spawns an open file dialog and loads a site accordingly.
    pub load_site_from_dialog: Service<(), ()>,
    /// Service that spawns a save file dialog then creates a site with an empty level.
    pub create_empty_site_from_dialog: Service<(), ()>,
    /// Loads the workspace at the requested path
    pub load_site_from_path: Service<PathBuf, ()>,
    /// Loads the workspace from the requested data
    pub load_site: Service<LoadSiteResult, ()>,
    /// Service that lets the user select a file to import nav graphs from.
    pub import_nav_graphs_from_dialog: Service<(), ()>,
}

impl FromWorld for SiteLoadingServices {
    fn from_world(world: &mut World) -> Self {
        let send_loaded_data = world.spawn_service(send_load_workspace_files);
        let pick_file = world
            .resource::<FileDialogServices>()
            .pick_file_and_read
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
                extensions: vec!["ron".into(), "json".into(), "yaml".into()],
            },
            FileDialogFilter {
                name: "All files".into(),
                extensions: vec!["*".into()],
            },
        ];
        // Spawn all the services
        let load_site_from_dialog = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_block({
                    let loading_filters = loading_filters.clone();
                    move |_| loading_filters.clone()
                })
                .then(pick_file)
                .map_async(|(path, data)| async move { LoadSite::from_data(&data, Some(path)) })
                .branch_for_err(|chain: Chain<_>| {
                    chain
                        .map_block(|err| error!("Failed to parse file: {err}"))
                        .connect(scope.terminate)
                })
                .then(send_loaded_data)
                .connect(scope.terminate)
        });

        let create_empty_site_from_dialog = world.spawn_workflow(|scope, builder| {
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
                            return Some(LoadSite::blank_L1(name, Some(file.clone())));
                        }
                        None
                    }
                    #[cfg(target_arch = "wasm32")]
                    {
                        let load_site = LoadSite::blank_L1("blank".to_owned(), None);
                        Some(load_site)
                    }
                })
                .cancel_on_none()
                .then(send_loaded_data)
                .connect(scope.terminate)
        });

        let load_site_from_path = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .map_async(|path| async move {
                    match std::fs::read(&path) {
                        Ok(data) => match LoadSite::from_data(&data, Some(path)) {
                            Ok(site) => Some(site),
                            Err(err) => {
                                warn!("Error parsing site data: {err}");
                                return None;
                            }
                        },
                        Err(err) => {
                            warn!("Cannot load file [{path:?}] because it cannot be read: {err}");
                            return None;
                        }
                    }
                })
                .cancel_on_none()
                .then(send_loaded_data)
                .connect(scope.terminate)
        });

        let load_site = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .branch_for_err(|chain: Chain<_>| {
                    chain
                        .map_block(|err| {
                            error!("Failed to load site: {err}");
                        })
                        .connect(scope.terminate)
                })
                .then(send_loaded_data)
                .connect(scope.terminate)
        });

        let request_import_nav_graphs_from_site =
            |In(from_site): In<Site>,
             current_site: Res<CurrentWorkspace>,
             mut import_nav_graphs: EventWriter<ImportNavGraphs>| {
                let Some(into_site) = current_site.root else {
                    return;
                };
                import_nav_graphs.write(ImportNavGraphs {
                    into_site,
                    from_site,
                });
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
                    LoadSite::from_data(&data, Some(path)).map(|load_site| load_site.site)
                })
                .branch_for_err(|chain: Chain<_>| {
                    chain
                        .map_block(|err| {
                            error!("Unable to import nav graphs from file: {err}");
                            error!(
                                "Nav graphs can only be imported from a legacy \
                            .building.yaml file or from a site file. We do not \
                            currently support importing from an exported nav \
                            graph file"
                            );
                        })
                        .connect(scope.terminate);
                })
                .then(request_import_nav_graphs_from_site.into_blocking_callback())
                .connect(scope.terminate);
        });

        Self {
            load_site_from_dialog,
            create_empty_site_from_dialog,
            load_site_from_path,
            load_site,
            import_nav_graphs_from_dialog,
        }
    }
}

impl<'w, 's> WorkspaceLoader<'w, 's> {
    /// Request to spawn a dialog and load a workspace
    pub fn load_from_dialog(&mut self) {
        self.commands
            .request((), self.workspace_loading.load_site_from_dialog)
            .detach();
    }

    /// Request to spawn a dialog to select a file and create a new site with a blank level
    pub fn create_empty_from_dialog(&mut self) {
        self.commands
            .request((), self.workspace_loading.create_empty_site_from_dialog)
            .detach();
    }

    /// Request to load a workspace from a path
    pub fn load_from_path(&mut self, path: PathBuf) -> Promise<()> {
        self.commands
            .request(path, self.workspace_loading.load_site_from_path)
            .detach()
            .take_response()
    }

    /// Request to load a workspace from data.
    ///
    /// This expects to receive a future to enforce the practice that
    /// LoadSite::from_data should be run async.
    pub fn load_site(
        &mut self,
        loader: impl Future<Output = LoadSiteResult> + Send + Sync + 'static,
    ) {
        self.commands
            .serve(loader)
            .then(self.workspace_loading.load_site)
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
    workspace_loading: Res<'w, SiteLoadingServices>,
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
    /// Exports Occupancy Grid from the requested
    pub export_occupancy_grid: Service<(), ()>,
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

        let export_occupancy_grid = world.spawn_workflow(|scope, builder| {
            scope
                .input
                .chain(builder)
                .then(pick_folder)
                .map_block(|path| (path, ExportFormat::OccupancyGrid))
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
            export_occupancy_grid,
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

    pub fn export_nav_graphs_to_path(&mut self, path: PathBuf) {
        self.commands
            .request(path, self.workspace_saving.export_nav_graphs_to_path)
            .detach();
    }

    /// Request to export the occupancy grid
    pub fn export_occupancy_to_dialog(&mut self) {
        self.commands
            .request((), self.workspace_saving.export_occupancy_grid)
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
