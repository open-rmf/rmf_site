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

use super::demo_world::demo_office;
use crate::{interaction::InteractionState, site::LoadSite, AppState, OpenedMapFile};
use bevy::{
    app::AppExit,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_egui::{egui, EguiContext};
use futures_lite::future;
#[cfg(not(target_arch = "wasm32"))]
use rfd::{AsyncFileDialog, FileHandle};
use rmf_site_format::{legacy::building_map::BuildingMap, Site};
use std::path::PathBuf;

struct LoadSiteFileResult(Option<OpenedMapFile>, Site);

#[derive(Component)]
struct LoadSiteFileTask(Task<Option<LoadSiteFileResult>>);

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

fn egui_ui(
    mut egui_context: ResMut<EguiContext>,
    mut _commands: Commands,
    mut _exit: EventWriter<AppExit>,
    mut _load_site: EventWriter<LoadSite>,
    mut _interaction_state: ResMut<State<InteractionState>>,
    mut _app_state: ResMut<State<AppState>>,
    autoload: Option<ResMut<Autoload>>,
    loading_tasks: Query<(), With<LoadSiteFileTask>>,
) {
    if !loading_tasks.is_empty() {
        egui::Window::new("Welcome!")
            .collapsible(false)
            .resizable(false)
            .title_bar(false)
            .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
            .show(egui_context.ctx_mut(), |ui| {
                ui.heading("Loading...");
            });
        return;
    }

    if let Some(mut autoload) = autoload {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(filename) = autoload.filename.clone() {
                let future = AsyncComputeTaskPool::get().spawn(async move {
                    let site = load_site_file(&FileHandle::wrap(filename.clone())).await?;
                    Some(LoadSiteFileResult(Some(OpenedMapFile(filename)), site))
                });
                _commands.spawn(LoadSiteFileTask(future));
            }
            autoload.filename = None;
        }
        return;
    }

    egui::Window::new("Welcome!")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
        .show(egui_context.ctx_mut(), |ui| {
            ui.heading("Welcome to The RMF Site Editor!");
            ui.add_space(10.);

            ui.horizontal(|ui| {
                if ui.button("View demo map").clicked() {
                    // load the office demo that is hard-coded in demo_world.rs
                    #[cfg(not(target_arch = "wasm32"))]
                    {
                        let future = AsyncComputeTaskPool::get().spawn(async move {
                            let yaml = demo_office();
                            let data = yaml.as_bytes();
                            let site = match BuildingMap::from_bytes(&data) {
                                Ok(building) => match building.to_site() {
                                    Ok(site) => site,
                                    Err(err) => {
                                        println!("{err:?}");
                                        return None;
                                    }
                                },
                                Err(err) => {
                                    println!("{:?}", err);
                                    return None;
                                }
                            };
                            Some(LoadSiteFileResult(None, site))
                        });
                        _commands.spawn(LoadSiteFileTask(future));
                    }

                    // on web, we don't have a handy thread pool, so we'll
                    // just parse the map here in the main thread.
                    #[cfg(target_arch = "wasm32")]
                    {
                        let yaml = demo_office();
                        let data = yaml.as_bytes();
                        match BuildingMap::from_bytes(&data) {
                            Ok(building) => match building.to_site() {
                                Ok(site) => {
                                    _load_site.send(LoadSite {
                                        site,
                                        focus: true,
                                        default_file: None,
                                    });
                                    match _app_state.set(AppState::SiteEditor) {
                                        Ok(_) => {
                                            _interaction_state.set(InteractionState::Enable).ok();
                                        }
                                        Err(err) => {
                                            println!("Failed to enter traffic editor: {:?}", err);
                                        }
                                    }
                                }
                                Err(err) => {
                                    println!("{err:?}");
                                }
                            },
                            Err(err) => {
                                println!("{:?}", err);
                            }
                        }
                    }

                    // switch to using a channel to signal completing the task
                }

                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui.button("Open a map file").clicked() {
                        // load the map in a thread pool
                        let future = AsyncComputeTaskPool::get().spawn(async move {
                            let file = match AsyncFileDialog::new().pick_file().await {
                                Some(file) => file,
                                None => {
                                    println!("No file selected");
                                    return None;
                                }
                            };
                            println!("Loading site map");

                            let site = load_site_file(&file).await?;
                            Some(LoadSiteFileResult(
                                Some(OpenedMapFile(file.path().to_path_buf())),
                                site,
                            ))
                        });
                        _commands.spawn(LoadSiteFileTask(future));
                    }
                }

                // TODO(MXG): Bring this back when we have time to fix the
                // warehouse generator.
                // if ui.button("Warehouse generator").clicked() {
                //     println!("Entering warehouse generator");
                //     _app_state.set(AppState::WarehouseGenerator).unwrap();
                // }
            });

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

/// Handles the file opening events
#[cfg(not(target_arch = "wasm32"))]
fn site_file_load_complete(
    mut commands: Commands,
    mut tasks: Query<(Entity, &mut LoadSiteFileTask)>,
    mut app_state: ResMut<State<AppState>>,
    mut interaction_state: ResMut<State<InteractionState>>,
    mut load_site: EventWriter<LoadSite>,
) {
    for (entity, mut task) in tasks.iter_mut() {
        if let Some(result) = future::block_on(future::poll_once(&mut task.0)) {
            println!("Site map loaded");
            commands.entity(entity).despawn();

            match result {
                Some(result) => {
                    println!("Entering traffic editor");
                    match app_state.set(AppState::SiteEditor) {
                        Ok(_) => {
                            let LoadSiteFileResult(file, site) = result;
                            load_site.send(LoadSite {
                                site,
                                focus: true,
                                default_file: file.map(|f| f.0),
                            });
                            interaction_state.set(InteractionState::Enable).ok();
                        }
                        Err(err) => {
                            println!("Failed to enter traffic editor: {:?}", err);
                        }
                    }
                }
                None => {}
            }
        }
    }
}

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_update(AppState::MainMenu).with_system(egui_ui));

        #[cfg(not(target_arch = "wasm32"))]
        app.add_system_set(
            SystemSet::on_update(AppState::MainMenu).with_system(site_file_load_complete),
        );
    }
}

#[cfg(not(target_arch = "wasm32"))]
pub async fn load_site_file(file: &FileHandle) -> Option<Site> {
    let is_legacy = file.file_name().ends_with(".building.yaml");
    let data = file.read().await;
    if is_legacy {
        match BuildingMap::from_bytes(&data) {
            Ok(building) => match building.to_site() {
                Ok(site) => Some(site),
                Err(err) => {
                    println!("{:?}", err);
                    return None;
                }
            },
            Err(err) => {
                println!("{:?}", err);
                return None;
            }
        }
    } else {
        match Site::from_bytes(&data) {
            Ok(site) => Some(site),
            Err(err) => {
                println!("{:?}", err);
                return None;
            }
        }
    }
}
