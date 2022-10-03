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

use rmf_site_format::{Label, RecallLabel};
use bevy_egui::egui::Ui;

pub struct InspectLabel<'a> {
    title: &'a str,
    label: &'a Label,
    recall: &'a RecallLabel,
}

impl<'a> InspectLabel<'a> {
    pub fn new(
        title: &'a str,
        label: &'a Label,
        recall: &'a RecallLabel,
    ) -> Self {
        Self{title, label, recall}
    }

    pub fn show(self, ui: &mut Ui) -> Option<Label> {
        ui.horizontal(|ui| {
            let mut has_value = self.label.is_some();
            ui.checkbox(&mut has_value, self.title);
            if has_value {
                let mut assumed_value = self.label.as_ref().map(|x| x.clone()).unwrap_or_else(
                    || self.recall.value.as_ref().map(|x| x.clone()).unwrap_or_else(
                        || "<undefined>".to_string()
                    )
                );
                ui.text_edit_singleline(&mut assumed_value);

                let new_label = Label(Some(assumed_value));
                if new_label != *self.label {
                    Some(new_label)
                } else {
                    None
                }
            } else {
                let new_label = Label(None);
                if new_label != *self.label {
                    Some(new_label)
                } else {
                    None
                }
            }
        }).inner
    }
}
