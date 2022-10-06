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
use rmf_site_format::Side;

pub struct InspectSide<'a> {
    pub side: &'a mut Side,
}

impl<'a> InspectSide<'a> {
    pub fn new(side: &'a mut Side) -> Self {
        Self { side }
    }

    pub fn show(self, ui: &mut Ui) {
        let response = ui.button(self.side.label()).on_hover_text(format!(
            "Click to change to {}",
            self.side.opposite().label()
        ));

        if response.clicked() {
            *self.side = self.side.opposite();
        }
    }
}
