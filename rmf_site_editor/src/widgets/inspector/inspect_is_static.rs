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
use rmf_site_format::IsStatic;

pub struct InspectIsStatic {
    pub is_static: IsStatic,
}

impl InspectIsStatic {
    pub fn new(is_static: &IsStatic) -> Self {
        Self {
            is_static: *is_static,
        }
    }

    pub fn show(self, ui: &mut Ui) -> Option<IsStatic> {
        let mut new_is_static = self.is_static;
        ui.checkbox(&mut new_is_static.0, "Static")
            .on_hover_text("Static means the object cannot move in a simulation");

        if new_is_static != self.is_static {
            return Some(new_is_static);
        }

        None
    }
}
