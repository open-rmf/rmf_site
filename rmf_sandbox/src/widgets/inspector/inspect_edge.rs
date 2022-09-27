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
    site::{SiteID, Original},
    interaction::{ChangeMode, SelectAnchor, InteractionMode},
    widgets::{
        AppEvents,
        inspector::{
            InspectAnchorParams,
            InspectAnchorWidget,
        },
    },
};
use rmf_site_format::{
    Edge, Side,
};
use bevy::prelude::*;
use bevy_egui::{
    egui::{Grid, Ui},
};

pub struct InspectEdgeWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub entity: Entity,
    pub edge: &'a Edge<Entity>,
    pub original: Option<&'a Original<Edge<Entity>>>,
    pub anchor_params: &'a mut InspectAnchorParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
    pub left_right: bool,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectEdgeWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        entity: Entity,
        edge: &'a Edge<Entity>,
        original: Option<&'a Original<Edge<Entity>>>,
        anchor_params: &'a mut InspectAnchorParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self{entity, original, edge, anchor_params, events, left_right: false}
    }

    pub fn left_right(self) -> Self {
        Self{
            left_right: true,
            ..self
        }
    }

    pub fn start_text(&self) -> &str {
        if self.left_right {
            "Left"
        } else {
            "Start"
        }
    }

    pub fn end_text(&self) -> &str {
        if self.left_right {
            "Right"
        } else {
            "End"
        }
    }

    pub fn show(self, ui: &mut Ui) {
        let edge = if let Some(original) = self.original {
            if original.is_reverse_of(self.edge) {
                // The user is previewing a flipped edge. To avoid ugly
                // high frequency UI flipping, we will display the edge
                // in its original form until the user has committed to
                // the flip.
                &original.0
            } else {
                self.edge
            }
        } else {
            self.edge
        };

        Grid::new("inspect_edge").show(ui, |ui| {
            ui.label("");
            ui.label("ID");
            ui.label("");
            ui.label("x");
            ui.label("y");
            ui.end_row();

            ui.label(self.start_text());
            let start_response = InspectAnchorWidget::new(
                edge.start(),
                self.anchor_params,
                self.events,
            ).as_dependency().show(ui);
            ui.end_row();
            if start_response.replace {
                self.events.change_mode.send(ChangeMode::To(InteractionMode::SelectAnchor(
                    SelectAnchor::replace_side(self.entity, Side::Left).for_lane()
                )));
            }

            ui.label(self.end_text());
            let end_response = InspectAnchorWidget::new(
                edge.end(),
                self.anchor_params,
                self.events
            ).as_dependency().show(ui);
            ui.end_row();
            if end_response.replace {
                self.events.change_mode.send(ChangeMode::To(InteractionMode::SelectAnchor(
                    SelectAnchor::replace_side(self.entity, Side::Right).for_lane()
                )));
            }
        });
    }
}
