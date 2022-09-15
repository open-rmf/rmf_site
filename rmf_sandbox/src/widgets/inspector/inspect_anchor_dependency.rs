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
    site::SiteID,
    interaction::{Select, Hover},
    widgets::Icons,
};
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::{
    egui::Widget,
};

#[derive(SystemParam)]
pub struct InspectAnchorDependencyParams<'w, 's> {
    pub hover: EventWriter<'w, 's, Hover>,
    pub select: EventWriter<'w, 's, Select>,
    pub icons: Res<'w, Icons>,
    pub site_id: Query<'w, 's, &'static SiteID>,
}

pub struct InspectAnchorDependencyWidget<'a, 'w, 's> {
    pub anchor: Entity,
    pub params: &'a mut InspectAnchorDependencyParams<'w, 's>,
}

impl<'a, 'w, 's> InspectAnchorDependencyWidget<'a, 'w, 's> {
    pub fn new(
        anchor: Entity,
        params: &'a mut InspectAnchorDependencyParams<'w, 's>,
    ) -> Self {
        Self{anchor, params}
    }

    pub fn show(self, ui: &mut bevy_egui::egui::Ui) {
        let select_response = ui.image(
            self.params.icons.egui_select, [18., 18.]
        );

        if select_response.clicked() {
            self.params.select.send(Select(Some(self.anchor)));
        } else if select_response.hovered() {
            self.params.hover.send(Hover(Some(self.anchor)));
        }

        if let Ok(site_id) = self.params.site_id.get(self.anchor) {
            ui.label(format!("Saved ID: {}", site_id.0));
        } else {
            ui.label("Not saved yet");
        }
    }
}
