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
    widgets::{
        AppEvents,
        inspector::{
            InspectAnchorParams,
            InspectAnchorWidget,
        },
    },
};
use rmf_site_format::{
    Lane,
};
use bevy::prelude::*;
use bevy_egui::{
    egui::{
        Widget, Grid, Ui,
    },
};

pub struct InspectLaneWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub lane: &'a Lane<Entity>,
    pub site_id: Option<&'a SiteID>,
    pub anchor_params: &'a mut InspectAnchorParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectLaneWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        lane: &'a Lane<Entity>,
        site_id: Option<&'a SiteID>,
        anchor_params: &'a mut InspectAnchorParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self{lane, site_id, anchor_params, events}
    }

    pub fn show(self, ui: &mut Ui) {
        Grid::new("inspect_lane").show(ui, |ui| {
            ui.label("");
            ui.label("ID");
            ui.label("");
            ui.label("");
            ui.label("x");
            ui.label("y");
            ui.end_row();

            ui.label("Start");
            InspectAnchorWidget::new(
                self.lane.anchors.0,
                self.anchor_params,
                self.events,
            ).as_dependency().show(ui);
            ui.end_row();

            ui.label("End");
            InspectAnchorWidget::new(
                self.lane.anchors.1,
                self.anchor_params,
                self.events
            ).as_dependency().show(ui);
            ui.end_row();
        });
    }
}
