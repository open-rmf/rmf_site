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
    site::{Anchor, SiteID},
};
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::{
    egui::{
        Widget, Label
    },
};

#[derive(SystemParam)]
pub struct InspectAnchorParams<'w, 's> {
    pub anchors: Query<'w, 's, Option<&'static SiteID>, With<Anchor>>,
}

pub struct InspectAnchorWidget<'a, 'w, 's> {
    pub anchor: Entity,
    pub params: &'a mut InspectAnchorParams<'w, 's>,
}

impl<'a, 'w, 's> InspectAnchorWidget<'a, 'w, 's> {
    pub fn new(
        anchor: Entity,
        params: &'a mut InspectAnchorParams<'w, 's>,
    ) -> Self {
        Self{anchor, params}
    }
}

impl<'a, 'w, 's> Widget for InspectAnchorWidget<'a, 'w, 's> {
    fn ui(self, ui: &mut bevy_egui::egui::Ui) -> bevy_egui::egui::Response {
        if let Ok(site_id) = self.params.anchors.get(self.anchor) {
            if let Some(site_id) = site_id {
                ui.label(format!("Site ID: {}", site_id.0))
            } else {
                ui.label("No Site ID")
            }
        } else {
            ui.label("Not an anchor??")
        }
    }
}
