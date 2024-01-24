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
use rmf_site_format::{NameInSite, NameInWorkcell, NameOfWorkcell};

// TODO(luca) refactor all these into a generic name inspection widget
pub struct InspectName<'a> {
    pub name: &'a NameInSite,
}

impl<'a> InspectName<'a> {
    pub fn new(name: &'a NameInSite) -> Self {
        Self { name }
    }

    pub fn show(self, ui: &mut Ui) -> Option<NameInSite> {
        ui.horizontal(|ui| {
            ui.label("Name");
            let mut new_name = self.name.clone();
            ui.text_edit_singleline(&mut new_name.0);
            if new_name != *self.name {
                Some(new_name)
            } else {
                None
            }
        })
        .inner
    }
}

pub struct InspectNameInWorkcell<'a> {
    pub name: &'a NameInWorkcell,
}

impl<'a> InspectNameInWorkcell<'a> {
    pub fn new(name: &'a NameInWorkcell) -> Self {
        Self { name }
    }

    pub fn show(self, ui: &mut Ui) -> Option<NameInWorkcell> {
        ui.horizontal(|ui| {
            ui.label("Name");
            let mut new_name = self.name.clone();
            ui.text_edit_singleline(&mut new_name.0);
            if new_name != *self.name {
                Some(new_name)
            } else {
                None
            }
        })
        .inner
    }
}

pub struct InspectNameOfWorkcell<'a> {
    pub name: &'a NameOfWorkcell,
}

impl<'a> InspectNameOfWorkcell<'a> {
    pub fn new(name: &'a NameOfWorkcell) -> Self {
        Self { name }
    }

    pub fn show(self, ui: &mut Ui) -> Option<NameOfWorkcell> {
        ui.horizontal(|ui| {
            ui.label("Name");
            let mut new_name = self.name.clone();
            ui.text_edit_singleline(&mut new_name.0);
            if new_name != *self.name {
                Some(new_name)
            } else {
                None
            }
        })
        .inner
    }
}
