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

use bevy_egui::egui::Ui;

pub struct InspectOptionString<'a> {
    title: &'a str,
    value: &'a Option<String>,
    recall: &'a Option<String>,
    default: &'a str,
    multiline: bool,
}

impl<'a> InspectOptionString<'a> {
    pub fn new(title: &'a str, value: &'a Option<String>, recall: &'a Option<String>) -> Self {
        Self {
            title,
            value,
            recall,
            default: "<undefined>",
            multiline: false,
        }
    }

    pub fn multiline(mut self) -> Self {
        self.multiline = true;
        self
    }

    pub fn default(mut self, default: &'a str) -> Self {
        self.default = default;
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<Option<String>> {
        ui.horizontal(|ui| {
            let mut has_value = self.value.is_some();
            ui.checkbox(&mut has_value, self.title);
            if has_value {
                let mut assumed_value =
                    self.value.as_ref().map(|x| x.clone()).unwrap_or_else(|| {
                        self.recall
                            .as_ref()
                            .map(|x| x.clone())
                            .unwrap_or_else(|| self.default.to_owned())
                    });
                if self.multiline {
                    ui.text_edit_multiline(&mut assumed_value);
                } else {
                    ui.text_edit_singleline(&mut assumed_value);
                }

                let new_value = Some(assumed_value);
                if new_value != *self.value {
                    Some(new_value)
                } else {
                    None
                }
            } else {
                let new_value = None;
                if new_value != *self.value {
                    Some(new_value)
                } else {
                    None
                }
            }
        })
        .inner
    }
}
