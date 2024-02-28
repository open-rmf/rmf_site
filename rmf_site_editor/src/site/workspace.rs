use bevy::{prelude::*, tasks::AsyncComputeTaskPool};
use rfd::AsyncFileDialog;
use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::site::{DefaultFile, LoadSite, SaveSite};
use crate::workspace::*;
use rmf_site_format::legacy::building_map::BuildingMap;
use rmf_site_format::{Level, NameOfSite, Site};

use bevy_file_dialog::prelude::*;
use crossbeam_channel::{Receiver, Sender};

/// Event used in channels to communicate the file handle that was chosen by the user.
#[derive(Debug)]
pub struct SaveWorkspaceFile {
    path: PathBuf,
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

impl CurrentWorkspace {
    pub fn to_site(self, open_sites: &Query<Entity, With<NameOfSite>>) -> Option<Entity> {
        let site_entity = self.root?;
        open_sites.get(site_entity).ok()
    }
}

pub struct SiteWorkspacePlugin;

impl Plugin for SiteWorkspacePlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SaveWorkspaceChannels>().add_systems(
            Update,
            (
                dispatch_load_workspace_events,
                dispatch_export_workspace_events,
                dispatch_new_workspace_events,
                workspace_file_load_complete,
                workspace_file_save_complete,
            ),
        );
        #[cfg(not(target_arch = "wasm32"))]
        app.add_systems(Update, dispatch_save_workspace_events);
        app.add_plugins(
            FileDialogPlugin::new()
                .with_save_file::<Site>()
                .with_load_file::<Site>(),
        );
    }
}

pub fn dispatch_load_workspace_events(
    mut commands: Commands,
    mut load_workspace: EventReader<LoadWorkspace>,
) {
    if let Some(cmd) = load_workspace.read().last() {
        match cmd {
            LoadWorkspace::Dialog => {
                commands
                    .dialog()
                    .add_filter("Legacy building", &["building.yaml"])
                    .add_filter("Site", &["site.ron"])
                    .load_file::<Site>();
            }
        }
    }
}

pub fn dispatch_export_workspace_events(mut export_workspace: EventReader<ExportWorkspace>) {
    if export_workspace.read().last().is_some() {
        warn!("Site exporting is not implemented yet");
    }
}

/// Handles the file opening events
fn workspace_file_load_complete(
    mut load_site: EventWriter<LoadSite>,
    mut site_loaded: EventReader<DialogFileLoaded<Site>>,
) {
    if let Some(result) = site_loaded.read().last() {
        let default_file = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                result.path.clone()
            }
            #[cfg(target_arch = "wasm32")]
            {
                PathBuf::from(result.file_name.clone())
            }
        };
        send_load_site_event(&mut load_site, &default_file, &result.contents);
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
    let spawn_dialog = |root| {
        let sender = save_channels.sender.clone();
        AsyncComputeTaskPool::get()
            .spawn(async move {
                if let Some(file) = AsyncFileDialog::new().save_file().await {
                    let path = file.path().to_path_buf();
                    sender
                        .send(SaveWorkspaceFile { path, root })
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
                                root: ws_root,
                            })
                            .expect("Failed sending save request");
                    } else {
                        spawn_dialog(ws_root);
                    }
                }
                SaveWorkspaceDestination::Dialog => spawn_dialog(ws_root),
            }
        } else {
            warn!("Unable to save, no workspace loaded");
            return;
        }
    }
}

/// Handles the file saving events
fn workspace_file_save_complete(
    mut save_site: EventWriter<SaveSite>,
    save_channels: Res<SaveWorkspaceChannels>,
) {
    if let Ok(result) = save_channels.receiver.try_recv() {
        save_site.send(SaveSite {
            site: result.root,
            to_file: result.path,
        });
    }
}

pub fn dispatch_new_workspace_events(
    mut new_workspace: EventReader<CreateNewWorkspace>,
    mut load_site: EventWriter<LoadSite>,
) {
    if let Some(_cmd) = new_workspace.read().last() {
        let mut levels = BTreeMap::new();
        levels.insert(0, Level::default());
        load_site.send(LoadSite {
            site: Site {
                levels,
                ..default()
            },
            focus: true,
            default_file: None,
        });
    }
}

pub(crate) fn send_load_site_event(
    load_site: &mut EventWriter<LoadSite>,
    default_file: &Path,
    data: &[u8],
) {
    let Some(filename) = default_file.file_name().and_then(|f| f.to_str()) else {
        error!(
            "Failed extracting file name from path {}",
            default_file.to_string_lossy()
        );
        return;
    };
    if filename.ends_with(".building.yaml") {
        info!("Opening legacy building map file");
        match BuildingMap::from_bytes(data) {
            Ok(building) => match building.to_site() {
                Ok(site) => {
                    load_site.send(LoadSite {
                        site,
                        focus: true,
                        default_file: Some(default_file.into()),
                    });
                }
                Err(err) => {
                    error!("Failed converting to site {:?}", err);
                }
            },
            Err(err) => {
                error!("Failed loading legacy building {:?}", err);
            }
        }
    } else if filename.ends_with(".site.ron") {
        info!("Opening site file");
        match Site::from_bytes(data) {
            Ok(site) => {
                load_site.send(LoadSite {
                    site,
                    focus: true,
                    default_file: Some(default_file.into()),
                });
            }
            Err(err) => {
                error!("Failed loading site {:?}", err);
            }
        }
    } else {
        error!(
            "Unsupported file type loaded {}",
            default_file.to_string_lossy()
        );
    }
}
