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

use crate::site::DefaultFile;
use bevy_egui::egui::{ComboBox, Ui};
use pathdiff::diff_paths;
use rmf_site_format::{AssetSource, RecallAssetSource};

#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

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
        };
        ui.horizontal(|ui| {
            ui.label("Source");
            ComboBox::from_id_source("Asset Source")
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
                    ui.text_edit_singleline(name);
                });
            }
            AssetSource::Remote(uri) => {
                ui.text_edit_singleline(uri);
            }
            AssetSource::Search(name) => {
                ui.text_edit_singleline(name);
            }
            AssetSource::Package(path) => {
                ui.text_edit_singleline(path);
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
