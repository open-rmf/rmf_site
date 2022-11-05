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
    interaction::{ChangeMode, InteractionMode, SelectAnchor},
    site::{EdgeLabels, Original, SiteID, Category},
    widgets::{
        inspector::{InspectAnchorParams, InspectAnchorWidget},
        AppEvents,
    },
};
use bevy::prelude::*;
use bevy_egui::egui::{Grid, Ui};
use rmf_site_format::{Edge, Side};

pub struct InspectEdgeWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub entity: Entity,
    pub category: &'a Category,
    pub edge: &'a Edge<Entity>,
    pub original: Option<&'a Original<Edge<Entity>>>,
    pub labels: Option<&'a EdgeLabels>,
    pub anchor_params: &'a mut InspectAnchorParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectEdgeWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        entity: Entity,
        category: &'a Category,
        edge: &'a Edge<Entity>,
        original: Option<&'a Original<Edge<Entity>>>,
        labels: Option<&'a EdgeLabels>,
        anchor_params: &'a mut InspectAnchorParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            entity,
            category,
            edge,
            original,
            labels,
            anchor_params,
            events,
        }
    }

    pub fn start_text(&self) -> &'static str {
        self.labels.unwrap_or(&EdgeLabels::default()).start()
    }

    pub fn end_text(&self) -> &'static str {
        self.labels.unwrap_or(&EdgeLabels::default()).end()
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
            let start_response =
                InspectAnchorWidget::new(edge.start(), self.anchor_params, self.events)
                    .as_dependency()
                    .show(ui);
            ui.end_row();
            if start_response.replace {
                if let Some(request) = SelectAnchor::replace_side(
                    self.entity, Side::Left
                ).for_category(*self.category) {
                    self.events.change_mode.send(ChangeMode::To(request.into()));
                }
            }

            ui.label(self.end_text());
            let end_response =
                InspectAnchorWidget::new(edge.end(), self.anchor_params, self.events)
                    .as_dependency()
                    .show(ui);
            ui.end_row();
            if end_response.replace {
                if let Some(request) = SelectAnchor::replace_side(
                    self.entity, Side::Right
                ).for_category(*self.category) {
                    self.events.change_mode.send(ChangeMode::To(request.into()));
                }
            }
        });
    }
}
