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
    widgets::{Icons, AppEvents},
    interaction::{Hover, Select},
};
use bevy::{
    prelude::*,
};
use bevy_egui::{
    egui::{Ui, ImageButton},
};

pub struct SelectionWidget<'a, 'w, 's> {
    entity: Entity,
    icons: &'a Icons,
    events: &'a mut AppEvents<'w, 's>,
}

impl<'a, 'w, 's> SelectionWidget<'a, 'w, 's> {
    pub fn new(
        entity: Entity,
        icons: &'a Icons,
        events: &'a mut AppEvents<'w, 's>,
    ) -> Self {
        Self{entity, icons, events}
    }

    pub fn show(self, ui: &mut Ui) {
        let response = ui.add(
            ImageButton::new(
                self.icons.egui_select,
                [18., 18.],
            )
        );

        if response.clicked() {
            self.events.select.send(Select(Some(self.entity)));
        } else if response.hovered() {
            self.events.hover.send(Hover(Some(self.entity)));
        }

        response.on_hover_text("Select");
    }
}
