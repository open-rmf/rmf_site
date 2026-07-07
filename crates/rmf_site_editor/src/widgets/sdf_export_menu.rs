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

use crate::{AppState, WorkspaceSaver};
use bevy::{
    ecs::{hierarchy::ChildOf, system::SystemState},
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_egui::{egui, EguiContexts};
use futures_lite::future;
#[cfg(not(target_arch = "wasm32"))]
use rfd::AsyncFileDialog;
use rmf_site_egui::*;
use std::path::PathBuf;

/// Keeps track of which entity is associated to the export sdf button.
#[derive(Resource)]
pub struct SdfExportMenu {
    export_sdf: Entity,
    pub show_dialog: bool,
    pub use_custom_base: bool,
    pub custom_base_path: Option<PathBuf>,
    pub choosing_file: Option<Task<Option<PathBuf>>>,
}

impl SdfExportMenu {
    pub fn get(&self) -> Entity {
        self.export_sdf
    }
}

impl FromWorld for SdfExportMenu {
    fn from_world(world: &mut World) -> Self {
        let file_header = world.resource::<FileMenu>().get();
        let export_sdf = world
            .spawn((
                MenuItem::Text(TextMenuItem::new("Export Sdf").shortcut("Ctrl-E")),
                ChildOf(file_header),
            ))
            .id();

        SdfExportMenu {
            export_sdf,
            show_dialog: false,
            use_custom_base: false,
            custom_base_path: None,
            choosing_file: None,
        }
    }
}

fn handle_export_sdf_menu_events(
    mut menu_events: EventReader<MenuEvent>,
    mut sdf_menu: ResMut<SdfExportMenu>,
) {
    for event in menu_events.read() {
        if event.clicked() && event.source() == sdf_menu.get() {
            sdf_menu.show_dialog = true;
        }
    }
}

fn show_export_sdf_dialog(
    mut contexts: EguiContexts,
    mut sdf_menu: ResMut<SdfExportMenu>,
    mut workspace_saver: WorkspaceSaver,
) {
    if !sdf_menu.show_dialog {
        return;
    }

    let mut open = true;
    let mut start_export = false;
    let mut browse_file = false;
    let mut cancel_clicked = false;

    egui::Window::new("Export SDF Options")
        .open(&mut open)
        .collapsible(false)
        .resizable(false)
        .anchor(egui::Align2::CENTER_CENTER, [0.0, 0.0])
        .show(contexts.ctx_mut(), |ui| {
            ui.checkbox(&mut sdf_menu.use_custom_base, "Use custom base SDF file");
            
            if sdf_menu.use_custom_base {
                ui.horizontal(|ui| {
                    if ui.button("Browse...").clicked() {
                        browse_file = true;
                    }
                    if let Some(path) = &sdf_menu.custom_base_path {
                        ui.label(path.to_string_lossy().to_string());
                    } else {
                        ui.label("<no file chosen>");
                    }
                });
            }
            
            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Export").clicked() {
                    start_export = true;
                }
                if ui.button("Cancel").clicked() {
                    cancel_clicked = true;
                }
            });
        });

    if browse_file {
        if sdf_menu.choosing_file.is_some() {
            warn!("A file is already being chosen!");
        } else {
            let task = AsyncComputeTaskPool::get().spawn(async move {
                #[cfg(not(target_arch = "wasm32"))]
                {
                    let file = AsyncFileDialog::new()
                        .set_title("Select Custom Base SDF File")
                        .add_filter("SDF File", &["sdf", "world", "xml"])
                        .add_filter("All Files", &["*"])
                        .pick_file()
                        .await;
                    file.map(|f| f.path().to_path_buf())
                }
                #[cfg(target_arch = "wasm32")]
                {
                    warn!("File picking is not implemented in wasm");
                    None
                }
            });
            sdf_menu.choosing_file = Some(task);
        }
    }

    if start_export {
        let base_sdf = if sdf_menu.use_custom_base {
            sdf_menu.custom_base_path.clone()
        } else {
            None
        };
        workspace_saver.export_sdf_to_dialog(base_sdf);
        sdf_menu.show_dialog = false;
    }

    if !open || cancel_clicked {
        sdf_menu.show_dialog = false;
    }
}

fn resolve_sdf_base_file(
    mut sdf_menu: ResMut<SdfExportMenu>,
) {
    let mut resolved = false;
    if let Some(task) = &mut sdf_menu.choosing_file {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            resolved = true;
            if let Some(path) = result {
                sdf_menu.custom_base_path = Some(path);
            }
        }
    }
    if resolved {
        sdf_menu.choosing_file = None;
    }
}

#[derive(Default)]
pub struct SdfExportMenuPlugin {}

impl Plugin for SdfExportMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<SdfExportMenu>().add_systems(
            Update,
            (
                handle_export_sdf_menu_events.run_if(AppState::in_displaying_mode()),
                resolve_sdf_base_file.run_if(AppState::in_displaying_mode()),
                show_export_sdf_dialog.run_if(AppState::in_displaying_mode()),
            ),
        );
    }
}
