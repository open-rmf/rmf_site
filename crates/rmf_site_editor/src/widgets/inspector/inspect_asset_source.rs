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

use crate::{
    site::{Change, DefaultFile},
    widgets::prelude::*,
    CurrentWorkspace,
};
use bevy::prelude::*;
use bevy_egui::egui::{ComboBox, TextEdit, Ui};
use pathdiff::diff_paths;
use rmf_site_format::{Affiliation, AssetSource, RecallAssetSource};

#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;
use rmf_site_egui::WidgetSystem;

pub struct InspectAssetSourceComponent<'a> {
    pub source: &'a AssetSource,
    pub recall: &'a RecallAssetSource,
    pub default_file: Option<&'a DefaultFile>,
}

impl<'a> InspectAssetSourceComponent<'a> {
    pub fn new(
        source: &'a AssetSource,
        recall: &'a RecallAssetSource,
        default_file: Option<&'a DefaultFile>,
    ) -> Self {
        Self {
            source,
            recall,
            default_file,
        }
    }

    pub fn show(self, ui: &mut Ui) -> Option<AssetSource> {
        let mut new_source = self.source.clone();

        // TODO(luca) implement recall plugin
        let assumed_source = match self.source {
            AssetSource::Local(filename) => filename,
            AssetSource::Remote(uri) => uri,
            AssetSource::Search(name) => name,
            AssetSource::Package(path) => path,
            AssetSource::Memory(uri) => uri,
        };
        ui.horizontal(|ui| {
            ui.label("Source");
            ComboBox::from_id_salt("Asset Source")
                .selected_text(new_source.label())
                .show_ui(ui, |ui| {
                    for variant in &[
                        AssetSource::Local(assumed_source.clone()),
                        AssetSource::Remote(assumed_source.clone()),
                        AssetSource::Search(assumed_source.clone()),
                        AssetSource::Package(assumed_source.clone()),
                    ] {
                        ui.selectable_value(&mut new_source, variant.clone(), variant.label());
                    }
                    ui.end_row();
                });
        });
        ui.add_space(4.0);
        match &mut new_source {
            AssetSource::Local(name) => {
                let is_relative = if let Some(default_file) = self.default_file {
                    let path = std::path::Path::new(name);
                    let mut is_relative = path.is_relative();
                    if ui.checkbox(&mut is_relative, "Relative").clicked() {
                        if is_relative {
                            let parent_dir = default_file
                                .0
                                .parent()
                                .map(|p| p.to_str())
                                .flatten()
                                .unwrap_or("");
                            if let Some(new_path) = diff_paths(path, parent_dir) {
                                if let Some(new_path) = new_path.to_str() {
                                    *name = new_path.to_owned();
                                }
                            }
                        } else {
                            if let Some(new_path) = default_file.with_file_name(path).to_str() {
                                *name = new_path.to_owned();
                            }
                        }
                    }
                    is_relative
                } else {
                    false
                };

                ui.horizontal(|ui| {
                    // Button to load from file, disabled for wasm since there are no local files
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Browse").clicked() {
                        // TODO(luca) change this to use FileDialogServices and be async
                        // https://github.com/open-rmf/rmf_site/issues/248
                        if let Some(file) = FileDialog::new().pick_file() {
                            if let Some(src) = file.to_str() {
                                if let (Some(default_file), true) = (self.default_file, is_relative)
                                {
                                    let parent_dir = default_file
                                        .0
                                        .parent()
                                        .map(|p| p.to_str())
                                        .flatten()
                                        .unwrap_or("");
                                    if let Some(buf) = diff_paths(src, parent_dir) {
                                        *name = buf.to_str().unwrap_or(src).to_owned();
                                    } else {
                                        *name = src.to_owned();
                                    }
                                } else {
                                    *name = src.to_owned();
                                }
                            }
                        };
                    }
                    TextEdit::singleline(name)
                        .desired_width(ui.available_width())
                        .show(ui);
                });
            }
            AssetSource::Remote(uri) => {
                TextEdit::singleline(uri)
                    .desired_width(ui.available_width())
                    .show(ui);
            }
            AssetSource::Search(name) => {
                TextEdit::singleline(name)
                    .desired_width(ui.available_width())
                    .show(ui);
            }
            AssetSource::Package(path) => {
                TextEdit::singleline(path)
                    .desired_width(ui.available_width())
                    .show(ui);
            }
            AssetSource::Memory(uri) => {
                ui.text_edit_singleline(uri);
            }
        }
        ui.add_space(10.0);

        if &new_source != self.source {
            Some(new_source)
        } else {
            None
        }
    }
}

#[derive(SystemParam)]
pub struct InspectAssetSource<'w, 's> {
    commands: Commands<'w, 's>,
    query: Query<
        'w,
        's,
        (&'static AssetSource, &'static RecallAssetSource),
        Without<Affiliation<Entity>>,
    >,
    default_file: Query<'w, 's, &'static DefaultFile>,
    current_workspace: Res<'w, CurrentWorkspace>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectAssetSource<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) -> () {
        let mut params = state.get_mut(world);
        let Ok((source, recall)) = params.query.get(selection) else {
            return;
        };
        let default_file = params
            .current_workspace
            .root
            .map(|e| params.default_file.get(e).ok())
            .flatten();

        if let Some(new_source) =
            InspectAssetSourceComponent::new(source, recall, default_file).show(ui)
        {
            params.commands.trigger(Change::new(new_source, selection));
        }
    }
}
