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
    interaction::{Hover, Select},
    site::SiteID,
    widgets::{AppEvents, Icons},
};
use bevy::prelude::*;
use bevy_egui::egui::{Button, Ui};

pub struct SelectionWidget<'a, 'w, 's> {
    entity: Entity,
    site_id: Option<SiteID>,
    icons: &'a Icons,
    events: &'a mut AppEvents<'w, 's>,
}

impl<'a, 'w, 's> SelectionWidget<'a, 'w, 's> {
    pub fn new(
        entity: Entity,
        site_id: Option<SiteID>,
        icons: &'a Icons,
        events: &'a mut AppEvents<'w, 's>,
    ) -> Self {
        Self {
            entity,
            site_id,
            icons,
            events,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let text = match self.site_id {
            Some(id) => format!("#{}", id.0),
            None => "*".to_string(),
        };

        let response = ui.add(Button::image_and_text(
            self.icons.egui_select,
            [18., 18.],
            text,
        ));

        if response.clicked() {
            self.events.request.select.send(Select(Some(self.entity)));
        } else if response.hovered() {
            self.events.request.hover.send(Hover(Some(self.entity)));
        }

        response.on_hover_text("Select");
    }
}
