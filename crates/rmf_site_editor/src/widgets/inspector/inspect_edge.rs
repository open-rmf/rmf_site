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
    interaction::AnchorSelection,
    site::{Category, EdgeLabels, Original},
    widgets::{
        inspector::{Inspect, InspectAnchor, InspectAnchorInput},
        prelude::*,
    },
};
use bevy::prelude::*;
use bevy_egui::egui::{Grid, Ui};
use rmf_site_egui::*;
use rmf_site_format::{Edge, Side};

#[derive(SystemParam)]
pub struct InspectEdge<'w, 's> {
    edges: Query<
        'w,
        's,
        (
            &'static Category,
            &'static Edge,
            Option<&'static Original<Edge>>,
            Option<&'static EdgeLabels>,
        ),
    >,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> ShareableWidget for InspectEdge<'w, 's> {}

impl<'w, 's> WidgetSystem<Inspect> for InspectEdge<'w, 's> {
    fn show(
        Inspect {
            selection: id,
            panel,
            ..
        }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
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

            Self::show_anchor(
                Side::Left,
                id,
                edge,
                labels,
                category,
                panel,
                ui,
                state,
                world,
            );
            Self::show_anchor(
                Side::Right,
                id,
                edge,
                labels,
                category,
                panel,
                ui,
                state,
                world,
            );
        });
        ui.add_space(10.0);
    }
}

impl<'w, 's> InspectEdge<'w, 's> {
    fn show_anchor(
        side: Side,
        id: Entity,
        edge: Edge,
        labels: EdgeLabels,
        category: Category,
        panel: PanelSide,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        ui.label(labels.side(side));
        let anchor = edge.side(side);
        let response = world.show::<InspectAnchor, _, _>(
            InspectAnchorInput {
                anchor,
                is_dependency: true,
                panel,
            },
            ui,
        );
        ui.end_row();

        match response {
            Some(response) => {
                if response.replace {
                    let mut params = state.get_mut(world);
                    if params.anchor_selection.replace_side(id, side, category) {
                        info!(
                            "Triggered anchor replacement for side \
                            {side:?} of edge {edge:?} with category {category:?}"
                        );
                    } else {
                        error!(
                            "Invalid type of element for replace_side operation: {category:?} \
                            Please report this error to the site editor maintainers."
                        );
                    }
                }
            }
            None => {
                error!(
                    "An endpoint in the edge {id:?} s not an \
                    anchor: {anchor:?}! This should never happen! Please report \
                    this to the site editor developers."
                );
            }
        }
    }
}
