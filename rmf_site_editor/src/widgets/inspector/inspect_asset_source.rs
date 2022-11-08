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

use bevy_egui::egui::{ComboBox, Grid, Ui};
use rmf_site_format::{DrawingSource, RecallDrawingSource};

use rfd::FileDialog;

// TODO DrawingSource -> AssetSource
pub struct InspectAssetSource<'a> {
    pub source: &'a DrawingSource,
    pub recall: &'a RecallDrawingSource,
}

impl<'a> InspectAssetSource<'a> {
    pub fn new(source: &'a DrawingSource, recall: &'a RecallDrawingSource) -> Self {
        Self { source, recall }
    }

    pub fn show(self, ui: &mut Ui) -> Option<DrawingSource> {
        let mut new_source = self.source.clone();
        // TODO recall plugin once multiple sources exist
        let assumed_source = match self.source {
            DrawingSource::Filename(filename) => filename
        };
        ui.vertical(|ui| {
            ui.label("Source");
            ComboBox::from_id_source("Asset Source")
                .selected_text(new_source.label())
                .show_ui(ui, |ui| {
                    for variant in &[
                        DrawingSource::Filename(assumed_source.clone()),
                    ] {
                        ui.selectable_value(&mut new_source, variant.clone(), variant.label());
                    }
                });
            match &mut new_source {
                DrawingSource::Filename(name) => {
                    Grid::new("asset_source_filename").show(ui, |ui| {
                        ui.end_row();
                        ui.label("filename");
                        ui.text_edit_singleline(name);
                        // Button to load from file
                        if ui.button("Browse").clicked() {
                            if let Some(file) = FileDialog::new().pick_file() {
                                if let Some(src) = file.to_str() {
                                    *name = String::from(src);
                                }
                            };
                        }
                        ui.end_row();

                    });

                }
            }
        })
        .inner;
        if (&new_source != self.source) {
            Some(new_source)
        }
        else {
            None
        }

    }
}
