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
use crate::{interaction::InteractionState, site::LoadSite, AppState, LoadWorkspace, LoadWorkspaceFile, LoadWorkspaceFileTask, OpenedWorkspaceFile, OpenedMapFile, workcell::LoadWorkcell};
use bevy::{
    app::AppExit,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_egui::{egui, EguiContext};
use futures_lite::future;
#[cfg(not(target_arch = "wasm32"))]
use rfd::{AsyncFileDialog, FileHandle};
use rmf_site_format::{legacy::building_map::BuildingMap, Site, Workcell};
use std::path::PathBuf;

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
    // TODO(luca) refactor into LoadWorkspace?
    mut _load_site: EventWriter<LoadSite>,
    mut _load_workcell: EventWriter<LoadWorkcell>,
    mut _load_workspace: EventWriter<LoadWorkspace>,
    mut _interaction_state: ResMut<State<InteractionState>>,
    mut _app_state: ResMut<State<AppState>>,
    autoload: Option<ResMut<Autoload>>,
    //loading_tasks: Query<(), With<LoadSiteFileTask>>,
) {
    /*
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
    */

    if let Some(mut autoload) = autoload {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(filename) = autoload.filename.clone() {
                _load_workspace.send(LoadWorkspace::Path(filename));
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
                            let mut mock_file = PathBuf::new();
                            mock_file.push("demo.building.yaml");
                            //Some(LoadWorkspaceFile("demo.building.yaml", data))
                            Some(LoadWorkspaceFile(
                                OpenedWorkspaceFile(mock_file),
                                demo_office(),
                            ))
                        });
                        _commands.spawn(LoadWorkspaceFileTask(future));
                    }

                    // on web, we don't have a handy thread pool, so we'll
                    // just parse the map here in the main thread.
                    #[cfg(target_arch = "wasm32")]
                    {
                        let data = demo_office();
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
                    if ui.button("Open a file").clicked() {
                        _load_workspace.send(LoadWorkspace::Dialog);
                    }
                }

                // TODO(MXG): Bring this back when we have time to fix the
                // warehouse generator.
                // if ui.button("Warehouse generator").clicked() {
                //     println!("Entering warehouse generator");
                //     _app_state.set(AppState::WarehouseGenerator).unwrap();
                // }
                if ui.button("Workcell Editor").clicked() {
                    println!("Entering workcell editor");
                    let data = demo_workcell();
                    match Workcell::from_bytes(&data) {
                        Ok(workcell) =>  {
                            // TODO(luca) remove this, for testing
                            let mut path = std::path::PathBuf::new();
                            path.push("test.workcell.json");
                            _load_workcell.send(LoadWorkcell {
                                workcell,
                                focus: true,
                                default_file: Some(path),
                            });
                            match _app_state.set(AppState::WorkcellEditor) {
                                Ok(_) => {
                                    _interaction_state.set(InteractionState::Enable).ok();
                                }
                                Err(err) => {
                                    println!("Failed to enter workcell editor: {:?}", err);
                                }
                            }
                        },
                        Err(err) => {
                            println!("{:?}", err);
                        }
                    }
                }
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

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(SystemSet::on_update(AppState::MainMenu).with_system(egui_ui));
    }
}
