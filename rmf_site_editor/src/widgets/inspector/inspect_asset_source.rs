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

use bevy_egui::egui::{ComboBox, Ui};
use rmf_site_format::{AssetSource, RecallAssetSource};

#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

pub struct InspectAssetSource<'a> {
    pub source: &'a AssetSource,
    pub recall: &'a RecallAssetSource,
}

impl<'a> InspectAssetSource<'a> {
    pub fn new(source: &'a AssetSource, recall: &'a RecallAssetSource) -> Self {
        Self { source, recall }
    }

    pub fn show(self, ui: &mut Ui) -> Option<AssetSource> {
        let mut new_source = self.source.clone();
        // TODO recall plugin once multiple sources exist
        let assumed_source = match self.source {
            AssetSource::Local(filename) => filename,
            AssetSource::Remote(uri) => uri,
            AssetSource::Search(name) => name,
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
                    ] {
                        ui.selectable_value(&mut new_source, variant.clone(), variant.label());
                    }
                    ui.end_row();
                });
        });
        match &mut new_source {
            AssetSource::Local(name) => {
                ui.horizontal(|ui| {
                    // Button to load from file
                    // TODO implement async file loading in wasm
                    #[cfg(not(target_arch = "wasm32"))]
                    if ui.button("Browse").clicked() {
                        if let Some(file) = FileDialog::new().pick_file() {
                            if let Some(src) = file.to_str() {
                                *name = String::from(src);
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
        }
        if &new_source != self.source {
            Some(new_source)
        } else {
            None
        }
    }
}
