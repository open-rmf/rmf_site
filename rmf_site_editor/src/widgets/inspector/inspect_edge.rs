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
    interaction::{ChangeMode, SelectAnchor},
    site::{Category, EdgeLabels, Original, SiteID},
    widgets::{
        inspector::{
            Inspect, InspectAnchorParams, InspectAnchorWidget, ExInspectAnchor,
            InspectAnchorInput,
        },
        AppEvents, prelude::*,
    },
};
use bevy::prelude::*;
use bevy_egui::egui::{Grid, Ui};
use rmf_site_format::{Edge, Side};

#[derive(SystemParam)]
pub struct ExInspectEdge<'w, 's> {
    edges: Query<
        'w,
        's,
        (
            &'static Category,
            &'static Edge<Entity>,
            Option<&'static Original<Edge<Entity>>>,
            Option<&'static EdgeLabels>,
        ),
    >,
    change_mode: EventWriter<'w, ChangeMode>,
}

impl<'w, 's> ShareableWidget for ExInspectEdge<'w, 's> { }

impl<'w, 's> WidgetSystem<Inspect> for ExInspectEdge<'w, 's> {
    fn show(
        Inspect { selection: id, panel, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World
    ) {
        let params = state.get_mut(world);
        let Ok((category, current_edge, original, labels)) = params.edges.get(id) else {
            return;
        };

        let edge = if let Some(original) = original {
            if original.is_reverse_of(current_edge) {
                // The user is previewing a flipped edge. To avoid ugly high
                // frequency UI flipping, we will display the edge in its
                // original form until the user has committed to the flip.
                original.0
            } else {
                *current_edge
            }
        } else {
            *current_edge
        };

        let labels = labels.copied().unwrap_or_default();
        let category = *category;

        Grid::new("inspect_edge").show(ui, |ui| {
            ui.label("");
            ui.label("ID");
            ui.label("");
            ui.label("x");
            ui.label("y");
            ui.end_row();

            Self::show_anchor(Side::Left, id, edge, labels, category, panel, ui, state, world);
            Self::show_anchor(Side::Right, id, edge, labels, category, panel, ui, state, world);
        });
    }
}

impl<'w, 's> ExInspectEdge<'w, 's> {
    fn show_anchor(
        side: Side,
        id: Entity,
        edge: Edge<Entity>,
        labels: EdgeLabels,
        category: Category,
        panel: PanelSide,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        ui.label(labels.side(side));
        let anchor = edge.side(side);
        let response = world.show::<ExInspectAnchor, _, _>(
            InspectAnchorInput { anchor, is_dependency: true, panel }, ui,
        );
        ui.end_row();

        match response {
            Some(response) => {
                if response.replace {
                    if let Some(request) = SelectAnchor::replace_side(id, side).for_category(category) {
                        info!(
                            "Triggered anchor replacement for side \
                            {side:?} of edge {edge:?} with category {category:?}"
                        );
                        let mut params = state.get_mut(world);
                        params.change_mode.send(ChangeMode::To(request.into()));
                    } else {
                        error!(
                            "Failed to trigger an anchor replacement for side \
                            {side:?} of edge {edge:?} with category {category:?}"
                        );
                    }
                }
            }
            None => {
                error!(
                    "An endpoint in the edge {id:?} (Site ID {:?}) is not an \
                    anchor: {anchor:?}! This should never happen! Please report \
                    this to the site editor developers.",
                    world.get::<SiteID>(anchor),
                );
            }
        }

    }
}

pub struct InspectEdgeWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub entity: Entity,
    pub category: &'a Category,
    pub edge: &'a Edge<Entity>,
    pub original: Option<&'a Original<Edge<Entity>>>,
    pub labels: Option<&'a EdgeLabels>,
    pub anchor_params: &'a InspectAnchorParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectEdgeWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        entity: Entity,
        category: &'a Category,
        edge: &'a Edge<Entity>,
        original: Option<&'a Original<Edge<Entity>>>,
        labels: Option<&'a EdgeLabels>,
        anchor_params: &'a InspectAnchorParams<'w1, 's1>,
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
                if let Some(request) =
                    SelectAnchor::replace_side(self.entity, Side::Left).for_category(*self.category)
                {
                    self.events
                        .request
                        .change_mode
                        .send(ChangeMode::To(request.into()));
                }
            }

            ui.label(self.end_text());
            let end_response =
                InspectAnchorWidget::new(edge.end(), self.anchor_params, self.events)
                    .as_dependency()
                    .show(ui);
            ui.end_row();
            if end_response.replace {
                if let Some(request) = SelectAnchor::replace_side(self.entity, Side::Right)
                    .for_category(*self.category)
                {
                    self.events
                        .request
                        .change_mode
                        .send(ChangeMode::To(request.into()));
                }
            }
        });
    }
}
