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

use crate::{
    site::{Change, DefaultFile},
    AppState, CurrentWorkspace, WorkspaceSaver,
};
use bevy::{
    ecs::hierarchy::ChildOf,
    prelude::*,
    tasks::{AsyncComputeTaskPool, Task},
};
use bevy_egui::{egui, EguiContexts};
use futures_lite::future;
use pathdiff::diff_paths;
#[cfg(not(target_arch = "wasm32"))]
use rfd::AsyncFileDialog;
use rmf_site_egui::*;
use rmf_site_format::BaseSdf;
use std::path::{Path, PathBuf};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileChoiceTarget {
    LinkFile { is_relative: bool },
    EmbedXml,
}

/// Keeps track of which entities are associated to the export sdf menu items.
#[derive(Resource)]
pub struct SdfExportMenu {
    export_sdf: Entity,
    export_sdf_settings: Entity,
    pub show_settings_dialog: bool,
    pub last_file: PathBuf,
    pub last_xml: String,
    pub choosing_file: Option<(FileChoiceTarget, Task<Option<PathBuf>>)>,
}

impl SdfExportMenu {
    pub fn get(&self) -> Entity {
        self.export_sdf
    }

    pub fn settings(&self) -> Entity {
        self.export_sdf_settings
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
        let export_sdf_settings = world
            .spawn((
                MenuItem::Text(TextMenuItem::new("SDF Export Settings")),
                ChildOf(file_header),
            ))
            .id();

        SdfExportMenu {
            export_sdf,
            export_sdf_settings,
            show_settings_dialog: false,
            last_file: PathBuf::new(),
            last_xml: String::new(),
            choosing_file: None,
        }
    }
}

fn handle_export_sdf_menu_events(
    mut menu_events: EventReader<MenuEvent>,
    mut sdf_menu: ResMut<SdfExportMenu>,
    mut workspace_saver: WorkspaceSaver,
) {
    for event in menu_events.read() {
        if event.clicked() {
            if event.source() == sdf_menu.get() {
                workspace_saver.export_sdf_to_dialog();
            } else if event.source() == sdf_menu.settings() {
                sdf_menu.show_settings_dialog = true;
            }
        }
    }
}

fn show_sdf_export_settings_dialog(
    mut commands: Commands,
    mut contexts: EguiContexts,
    mut sdf_menu: ResMut<SdfExportMenu>,
    current_workspace: Res<CurrentWorkspace>,
    base_sdfs: Query<&BaseSdf>,
    default_files: Query<&DefaultFile>,
) {
    if !sdf_menu.show_settings_dialog {
        return;
    }

    let Some(ws_root) = current_workspace.root else {
        sdf_menu.show_settings_dialog = false;
        return;
    };

    let Ok(current_base_sdf) = base_sdfs.get(ws_root) else {
        return;
    };

    match current_base_sdf {
        BaseSdf::Default => {}
        BaseSdf::File(path) => {
            sdf_menu.last_file = path.clone();
        }
        BaseSdf::Xml(xml) => {
            sdf_menu.last_xml = xml.clone();
        }
    }

    let default_file = default_files.get(ws_root).ok();
    let mut new_base_sdf = current_base_sdf.clone();
    let mut open = true;
    let mut close_clicked = false;
    let mut file_choice_request: Option<FileChoiceTarget> = None;

    egui::Window::new("SDF Export Settings")
        .open(&mut open)
        .collapsible(false)
        .resizable(true)
        .default_width(450.0)
        .show(contexts.ctx_mut(), |ui| {
            ui.label("Base SDF World Template:");
            ui.horizontal(|ui| {
                if ui
                    .radio(matches!(new_base_sdf, BaseSdf::Default), "Default")
                    .clicked()
                {
                    new_base_sdf = BaseSdf::Default;
                }
                if ui
                    .radio(matches!(new_base_sdf, BaseSdf::File(_)), "Link File")
                    .clicked()
                {
                    new_base_sdf = BaseSdf::File(sdf_menu.last_file.clone());
                }
                if ui
                    .radio(matches!(new_base_sdf, BaseSdf::Xml(_)), "Embed Raw XML")
                    .clicked()
                {
                    if sdf_menu.last_xml.is_empty() && !sdf_menu.last_file.as_os_str().is_empty() {
                        let resolved = resolve_path(&sdf_menu.last_file, default_file);
                        if let Ok(xml) = std::fs::read_to_string(resolved) {
                            sdf_menu.last_xml = xml;
                        }
                    }
                    new_base_sdf = BaseSdf::Xml(sdf_menu.last_xml.clone());
                }
            });

            ui.separator();

            match &mut new_base_sdf {
                BaseSdf::Default => {
                    ui.label("Uses the built-in Gazebo world template when exporting to SDF.");
                }
                BaseSdf::File(path_buf) => {
                    let mut path_str = path_buf.to_string_lossy().to_string();
                    let is_relative = if let Some(default_file) = default_file {
                        let path = Path::new(&path_str);
                        let mut is_relative = path.is_relative();
                        if ui
                            .checkbox(&mut is_relative, "Relative to .site.json")
                            .clicked()
                        {
                            let parent_dir = default_file.0.parent().unwrap_or(Path::new(""));
                            if is_relative {
                                if let Some(rel) = diff_paths(path, parent_dir) {
                                    path_str = rel.to_string_lossy().into_owned();
                                }
                            } else {
                                path_str = parent_dir.join(path).to_string_lossy().into_owned();
                            }
                        }
                        is_relative
                    } else {
                        false
                    };

                    ui.horizontal(|ui| {
                        if ui.button("Browse...").clicked() {
                            file_choice_request = Some(FileChoiceTarget::LinkFile { is_relative });
                        }
                        egui::TextEdit::singleline(&mut path_str)
                            .hint_text("Path to base .sdf or .world file")
                            .desired_width(ui.available_width())
                            .show(ui);
                    });

                    *path_buf = PathBuf::from(path_str);
                }
                BaseSdf::Xml(xml) => {
                    ui.horizontal(|ui| {
                        if ui.button("Load from File...").clicked() {
                            file_choice_request = Some(FileChoiceTarget::EmbedXml);
                        }
                        ui.label("Embedded XML saved directly in .site.json");
                    });
                    ui.add_space(4.0);
                    egui::ScrollArea::vertical()
                        .max_height(250.0)
                        .show(ui, |ui| {
                            egui::TextEdit::multiline(xml)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .desired_rows(10)
                                .show(ui);
                        });
                }
            }

            ui.add_space(10.0);
            ui.horizontal(|ui| {
                if ui.button("Close").clicked() {
                    close_clicked = true;
                }
            });
        });

    if let Some(target) = file_choice_request {
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
            sdf_menu.choosing_file = Some((target, task));
        }
    }

    if &new_base_sdf != current_base_sdf {
        commands.trigger(Change::new(new_base_sdf, ws_root));
    }

    if !open || close_clicked {
        sdf_menu.show_settings_dialog = false;
    }
}

fn resolve_path(path: &Path, default_file: Option<&DefaultFile>) -> PathBuf {
    if path.is_relative() {
        if let Some(default_file) = default_file {
            if let Some(parent) = default_file.0.parent() {
                let candidate = parent.join(path);
                if candidate.exists() {
                    return candidate;
                }
            }
        }
    }
    path.to_path_buf()
}

fn resolve_sdf_base_file(
    mut commands: Commands,
    mut sdf_menu: ResMut<SdfExportMenu>,
    current_workspace: Res<CurrentWorkspace>,
    default_files: Query<&DefaultFile>,
) {
    let mut resolved = None;
    if let Some((target, task)) = &mut sdf_menu.choosing_file {
        if let Some(result) = future::block_on(future::poll_once(task)) {
            resolved = Some((*target, result));
        }
    }
    if let Some((target, maybe_path)) = resolved {
        sdf_menu.choosing_file = None;
        let Some(path) = maybe_path else {
            return;
        };
        let Some(ws_root) = current_workspace.root else {
            return;
        };
        match target {
            FileChoiceTarget::LinkFile { is_relative } => {
                let final_path = if is_relative {
                    if let Ok(default_file) = default_files.get(ws_root) {
                        let parent_dir = default_file.0.parent().unwrap_or(Path::new(""));
                        diff_paths(&path, parent_dir).unwrap_or(path)
                    } else {
                        path
                    }
                } else {
                    path
                };
                sdf_menu.last_file = final_path.clone();
                commands.trigger(Change::new(BaseSdf::File(final_path), ws_root));
            }
            FileChoiceTarget::EmbedXml => match std::fs::read_to_string(&path) {
                Ok(xml) => {
                    sdf_menu.last_xml = xml.clone();
                    commands.trigger(Change::new(BaseSdf::Xml(xml), ws_root));
                }
                Err(err) => {
                    error!("Unable to read base SDF file at {}: {err}", path.display());
                }
            },
        }
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
                show_sdf_export_settings_dialog.run_if(AppState::in_displaying_mode()),
            ),
        );
    }
}
