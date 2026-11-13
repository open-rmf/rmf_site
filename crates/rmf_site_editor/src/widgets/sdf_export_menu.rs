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
    site::{resolve_and_read_base_sdf_file, Change, DefaultFile},
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

/// Resolve a path to an absolute path (using `current_dir` if relative) so `diff_paths` can
/// reliably compute relative paths even when `.site.json` was loaded via a relative path.
fn make_absolute(path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else if let Ok(cwd) = std::env::current_dir() {
        cwd.join(path)
    } else {
        path.to_path_buf()
    }
}

fn make_relative_to_site(path: &Path, default_file: &DefaultFile) -> Option<PathBuf> {
    let parent_dir = default_file.0.parent().unwrap_or(Path::new(""));
    diff_paths(make_absolute(path), make_absolute(parent_dir))
}

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum FileChoiceTarget {
    LinkFile { is_relative: bool },
    EmbedXml,
}

#[derive(Clone, Copy, PartialEq, Eq, Default)]
pub enum SdfExportMode {
    #[default]
    Default,
    LinkFile,
    EmbedXml,
}

/// Keeps track of which entities are associated to the export sdf menu items.
#[derive(Resource)]
pub struct SdfExportMenu {
    export_sdf: Entity,
    export_sdf_settings: Entity,
    pub show_settings_dialog: bool,
    pub initialized_for_site: Option<Entity>,
    pub mode: SdfExportMode,
    pub last_file: PathBuf,
    pub file_validation: Result<(), String>,
    pub last_xml: String,
    pub xml_validation: Result<(), String>,
    pub load_xml_error: Option<String>,
    pub choosing_file: Option<(FileChoiceTarget, Task<Option<PathBuf>>)>,
}

impl SdfExportMenu {
    pub fn get(&self) -> Entity {
        self.export_sdf
    }

    pub fn settings(&self) -> Entity {
        self.export_sdf_settings
    }

    fn revalidate_file(
        &mut self,
        default_file: Option<&DefaultFile>,
        current_base_sdf: &BaseSdf,
        ws_root: Entity,
        commands: &mut Commands,
    ) {
        if self.last_file.as_os_str().is_empty() {
            self.file_validation = Err(String::new());
            return;
        }
        self.file_validation =
            resolve_and_read_base_sdf_file(&self.last_file, default_file).map(|_| ());
        if self.file_validation.is_ok() {
            let new_base = BaseSdf::File(self.last_file.clone());
            if &new_base != current_base_sdf {
                commands.trigger(Change::new(new_base, ws_root));
            }
        }
    }

    /// Re-validate the embedded XML string and apply the change to the workspace if valid.
    fn revalidate_xml(
        &mut self,
        current_base_sdf: &BaseSdf,
        ws_root: Entity,
        commands: &mut Commands,
    ) {
        if self.last_xml.trim().is_empty() {
            self.xml_validation = Err(String::new());
            return;
        }
        self.xml_validation = rmf_site_format::sdf::validate_base_sdf_xml(&self.last_xml)
            .map(|_| ())
            .map_err(|err| format!("Invalid base SDF XML: {err}"));
        if self.xml_validation.is_ok() {
            let new_base = BaseSdf::Xml(self.last_xml.clone());
            if &new_base != current_base_sdf {
                commands.trigger(Change::new(new_base, ws_root));
            }
        }
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
            initialized_for_site: None,
            mode: SdfExportMode::Default,
            last_file: PathBuf::new(),
            file_validation: Err(String::new()),
            last_xml: String::new(),
            xml_validation: Err(String::new()),
            load_xml_error: None,
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
                sdf_menu.initialized_for_site = None;
                sdf_menu.load_xml_error = None;
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

    let default_file = default_files.get(ws_root).ok();

    if sdf_menu.initialized_for_site != Some(ws_root) {
        match current_base_sdf {
            BaseSdf::Default => {
                sdf_menu.mode = SdfExportMode::Default;
            }
            BaseSdf::File(path) => {
                sdf_menu.mode = SdfExportMode::LinkFile;
                sdf_menu.last_file = path.clone();
                sdf_menu.revalidate_file(default_file, current_base_sdf, ws_root, &mut commands);
            }
            BaseSdf::Xml(xml) => {
                sdf_menu.mode = SdfExportMode::EmbedXml;
                sdf_menu.last_xml = xml.clone();
                sdf_menu.revalidate_xml(current_base_sdf, ws_root, &mut commands);
            }
        }
        sdf_menu.load_xml_error = None;
        sdf_menu.initialized_for_site = Some(ws_root);
    }

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
                    .radio(sdf_menu.mode == SdfExportMode::Default, "Default")
                    .clicked()
                {
                    sdf_menu.mode = SdfExportMode::Default;
                    if *current_base_sdf != BaseSdf::Default {
                        commands.trigger(Change::new(BaseSdf::Default, ws_root));
                    }
                }
                if ui
                    .radio(sdf_menu.mode == SdfExportMode::LinkFile, "Link File")
                    .clicked()
                {
                    sdf_menu.mode = SdfExportMode::LinkFile;
                    sdf_menu.revalidate_file(
                        default_file,
                        current_base_sdf,
                        ws_root,
                        &mut commands,
                    );
                }
                if ui
                    .radio(sdf_menu.mode == SdfExportMode::EmbedXml, "Embed Raw XML")
                    .clicked()
                {
                    if sdf_menu.last_xml.is_empty() && !sdf_menu.last_file.as_os_str().is_empty() {
                        if let Ok(xml) =
                            resolve_and_read_base_sdf_file(&sdf_menu.last_file, default_file)
                        {
                            sdf_menu.last_xml = xml;
                        }
                    }
                    sdf_menu.mode = SdfExportMode::EmbedXml;
                    sdf_menu.revalidate_xml(current_base_sdf, ws_root, &mut commands);
                }
            });

            ui.separator();

            match sdf_menu.mode {
                SdfExportMode::Default => {
                    ui.label("Uses the built-in Gazebo world template when exporting to SDF.");
                }
                SdfExportMode::LinkFile => {
                    let mut path_str = sdf_menu.last_file.to_string_lossy().to_string();
                    let mut needs_validation = false;
                    let is_relative = if let Some(default_file) = default_file {
                        let path = Path::new(&path_str);
                        let mut is_relative = !path.as_os_str().is_empty() && path.is_relative();
                        if ui
                            .checkbox(&mut is_relative, "Relative to .site.json")
                            .clicked()
                        {
                            let parent_dir = default_file.0.parent().unwrap_or(Path::new(""));
                            if !path.as_os_str().is_empty() {
                                if is_relative {
                                    if let Some(rel) = make_relative_to_site(path, default_file) {
                                        path_str = rel.to_string_lossy().into_owned();
                                        needs_validation = true;
                                    }
                                } else {
                                    path_str = parent_dir.join(path).to_string_lossy().into_owned();
                                    needs_validation = true;
                                }
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
                        let response = egui::TextEdit::singleline(&mut path_str)
                            .hint_text("Path to base .sdf or .world file")
                            .desired_width(ui.available_width())
                            .show(ui);
                        if response.response.changed() {
                            needs_validation = true;
                        }
                    });

                    if needs_validation {
                        sdf_menu.last_file = PathBuf::from(&path_str);
                        sdf_menu.revalidate_file(
                            default_file,
                            current_base_sdf,
                            ws_root,
                            &mut commands,
                        );
                    }

                    if path_str.trim().is_empty() {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            "Please enter or browse for a base SDF file.",
                        );
                    } else {
                        match &sdf_menu.file_validation {
                            Ok(()) => {
                                ui.colored_label(
                                    egui::Color32::LIGHT_GREEN,
                                    "Valid base SDF file.",
                                );
                            }
                            Err(err) => {
                                ui.colored_label(egui::Color32::LIGHT_RED, err.as_str());
                            }
                        }
                    }
                }
                SdfExportMode::EmbedXml => {
                    ui.horizontal(|ui| {
                        if ui.button("Load from File...").clicked() {
                            file_choice_request = Some(FileChoiceTarget::EmbedXml);
                        }
                        ui.label("Embedded XML saved directly in .site.json");
                    });
                    if let Some(err) = &sdf_menu.load_xml_error {
                        ui.colored_label(egui::Color32::LIGHT_RED, err);
                    }
                    ui.add_space(4.0);
                    let response = egui::ScrollArea::vertical()
                        .max_height(250.0)
                        .show(ui, |ui| {
                            egui::TextEdit::multiline(&mut sdf_menu.last_xml)
                                .code_editor()
                                .desired_width(f32::INFINITY)
                                .desired_rows(10)
                                .show(ui)
                        });
                    if response.inner.response.changed() {
                        sdf_menu.load_xml_error = None;
                        sdf_menu.revalidate_xml(current_base_sdf, ws_root, &mut commands);
                    }

                    if sdf_menu.last_xml.trim().is_empty() {
                        ui.colored_label(
                            egui::Color32::YELLOW,
                            "Please enter or load base SDF XML.",
                        );
                    } else {
                        match &sdf_menu.xml_validation {
                            Ok(()) => {
                                ui.colored_label(egui::Color32::LIGHT_GREEN, "Valid base SDF XML.");
                            }
                            Err(err) => {
                                ui.colored_label(egui::Color32::LIGHT_RED, err.as_str());
                            }
                        }
                    }
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

    if !open || close_clicked {
        sdf_menu.show_settings_dialog = false;
    }
}

fn resolve_sdf_base_file(
    mut commands: Commands,
    mut sdf_menu: ResMut<SdfExportMenu>,
    current_workspace: Res<CurrentWorkspace>,
    base_sdfs: Query<&BaseSdf>,
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
        let Ok(current_base_sdf) = base_sdfs.get(ws_root) else {
            return;
        };
        let default_file = default_files.get(ws_root).ok();
        match target {
            FileChoiceTarget::LinkFile { is_relative } => {
                let final_path = if is_relative {
                    if let Some(default_file) = default_file {
                        make_relative_to_site(&path, default_file).unwrap_or(path)
                    } else {
                        path
                    }
                } else {
                    path
                };
                sdf_menu.last_file = final_path;
                sdf_menu.revalidate_file(default_file, current_base_sdf, ws_root, &mut commands);
                if let Err(err) = &sdf_menu.file_validation {
                    error!("{err}");
                }
            }
            FileChoiceTarget::EmbedXml => match resolve_and_read_base_sdf_file(&path, None) {
                Ok(xml) => {
                    sdf_menu.last_xml = xml.clone();
                    sdf_menu.xml_validation = Ok(());
                    sdf_menu.load_xml_error = None;
                    commands.trigger(Change::new(BaseSdf::Xml(xml), ws_root));
                }
                Err(err) => {
                    error!("{err}");
                    sdf_menu.load_xml_error = Some(err);
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
