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
    widgets::inspector::{
        InspectAnchorDependencyParams,
        InspectAnchorDependencyWidget,
    },
};
use rmf_site_format::{
    Lane,
};
use bevy::prelude::*;
use bevy_egui::{
    egui::{
        Widget, Grid,
    },
};

pub struct InspectLaneWidget<'a, 'w, 's> {
    pub lane: &'a Lane<Entity>,
    pub site_id: Option<&'a SiteID>,
    pub anchor_params: &'a mut InspectAnchorDependencyParams<'w, 's>,
}

impl<'a, 'w, 's> InspectLaneWidget<'a, 'w, 's> {
    pub fn new(
        lane: &'a Lane<Entity>,
        site_id: Option<&'a SiteID>,
        anchor_params: &'a mut InspectAnchorDependencyParams<'w, 's>,
    ) -> Self {
        Self{lane, site_id, anchor_params}
    }
}

impl<'a, 'w, 's> Widget for InspectLaneWidget<'a, 'w, 's> {
    fn ui(self, ui: &mut bevy_egui::egui::Ui) -> bevy_egui::egui::Response {
        Grid::new("inspect_lane").show(ui, |ui| {
            ui.label("Start");
            InspectAnchorDependencyWidget::new(
                self.lane.anchors.0,
                self.anchor_params,
            ).show(ui);
            ui.end_row();

            ui.label("End");
            InspectAnchorDependencyWidget::new(
                self.lane.anchors.1,
                self.anchor_params,
            ).show(ui);
            ui.end_row();
        }).response
    }
}
