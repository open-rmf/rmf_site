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

pub mod inspect_anchor;
pub use inspect_anchor::*;

pub mod inspect_lane;
pub use inspect_lane::*;

pub mod selection_widget;
pub use selection_widget::*;

use crate::{
    site::{SiteID, Original},
    interaction::Selection,
    widgets::AppEvents,
};
use rmf_site_format::{
    LaneMarker, Edge,
};
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::{
    egui::{Label, Ui},
};

#[derive(SystemParam)]
pub struct InspectorParams<'w, 's> {
    pub selection: Res<'w, Selection>,
    pub site_id: Query<'w, 's, Option<&'static SiteID>>,
    pub anchor_params: InspectAnchorParams<'w, 's>,
    pub anchor_dependents_params: InspectAnchorDependentsParams<'w, 's>,
    pub lanes: Query<'w, 's, (
        &'static Edge<Entity>,
        Option<&'static Original<Edge<Entity>>>
    ), With<LaneMarker>>,
}

pub struct InspectorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub params: &'a mut InspectorParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        params: &'a mut InspectorParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self{params, events}
    }

    fn heading(label: &str, site_id: Option<&SiteID>, ui: &mut Ui) {
        if let Some(site_id) = site_id {
            ui.heading(format!("{} #{}", label, site_id.0));
        } else {
            ui.heading(format!("{} (unsaved)", label));
        }
    }
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn show(self, ui: &mut Ui) {
        if let Some(selection) =  self.params.selection.0 {
            let site_id = self.params.site_id.get(selection).ok().flatten();
            if self.params.anchor_params.transforms.contains(selection) {
                Self::heading("Anchor", site_id, ui);
                ui.horizontal(|ui| {
                    InspectAnchorWidget::new(
                        selection,
                        &mut self.params.anchor_params,
                        self.events,
                    ).show(ui);
                });
                ui.separator();
                InspectAnchorDependentsWidget::new(
                    selection,
                    &mut self.params.anchor_dependents_params,
                    self.events,
                ).show(ui);
            } else if let Ok((edge, o_edge)) = self.params.lanes.get(selection) {
                let edge = if let Some(original) = o_edge {
                    if original.is_reverse_of(edge) {
                        // The user is previewing a flipped edge. To avoid ugly
                        // high frequency UI flipping, we will display the edge
                        // in its original form until the user has committed to
                        // the flip.
                        &original.0
                    } else {
                        edge
                    }
                } else {
                    edge
                };

                Self::heading("Lane", site_id, ui);
                InspectLaneWidget::new(
                    selection, edge, site_id,
                    &mut self.params.anchor_params,
                    self.events,
                ).show(ui);
            } else {
                ui.add(
                    Label::new("Unsupported selection type")
                    .wrap(false)
                );
            }
        } else {
            ui.add(
                Label::new("Nothing selected")
                .wrap(false)
            );
        }
    }
}
