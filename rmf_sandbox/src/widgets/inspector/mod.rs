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

pub mod inspect_angle;
pub use inspect_angle::*;

pub mod inspect_edge;
pub use inspect_edge::*;

pub mod inspect_lane;
pub use inspect_lane::*;

pub mod inspect_option_f32;
pub use inspect_option_f32::*;

pub mod inspect_pose;
pub use inspect_pose::*;

pub mod selection_widget;
pub use selection_widget::*;

use crate::{
    site::{SiteID, Category, Original, Change},
    interaction::Selection,
    widgets::AppEvents,
};
use rmf_site_format::*;
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::{
    egui::{Label, Ui, RichText},
};

#[derive(SystemParam)]
pub struct InspectorParams<'w, 's> {
    pub selection: Res<'w, Selection>,
    pub heading: Query<'w, 's, (Option<&'static Category>, Option<&'static SiteID>)>,
    pub anchor_params: InspectAnchorParams<'w, 's>,
    pub anchor_dependents_params: InspectAnchorDependentsParams<'w, 's>,
    pub edges: Query<'w, 's, (&'static Edge<Entity>, Option<&'static Original<Edge<Entity>>>)>,
    pub motions: Query<'w, 's, (&'static Motion, &'static RecallMotion)>,
    pub reverse_motions: Query<'w, 's, (&'static ReverseLane, &'static RecallReverseLane)>,
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

    fn heading(&self, selection: Entity, ui: &mut Ui) {
        let (label, site_id) = if let Ok((category, site_id)) = self.params.heading.get(selection) {
            (
                category.map(|x| x.0.as_str()).unwrap_or("<Unknown Type>"),
                site_id,
            )
        } else {
            ("<Unknown Type>", None)
        };

        if let Some(site_id) = site_id {
            ui.heading(format!("{} #{}", label, site_id.0));
        } else {
            ui.heading(format!("{} (unsaved)", label));
        }
    }

    pub fn show(self, ui: &mut Ui) {
        if let Some(selection) =  self.params.selection.0 {
            self.heading(selection, ui);
            if self.params.anchor_params.transforms.contains(selection) {
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
                ui.add_space(10.0);
            }

            if let Ok((edge, original)) = self.params.edges.get(selection) {
                InspectEdgeWidget::new(
                    selection, edge, original, &mut self.params.anchor_params, self.events
                ).show(ui);
                ui.add_space(10.0);
            }

            if let Ok((motion, recall)) = self.params.motions.get(selection) {
                ui.label(RichText::new("Forward Motion").size(18.0));
                if let Some(new_motion) = InspectMotionWidget::new(motion, recall).show(ui) {
                    self.events.change_lane_motion.send(Change::new(new_motion, selection));
                }
                ui.add_space(10.0);
            }

            if let Ok((reverse, recall)) = self.params.reverse_motions.get(selection) {
                ui.separator();
                ui.push_id("Reverse Motion", |ui| {
                    if let Some(new_reverse) = InspectReverseWidget::new(reverse, recall).show(ui) {
                        self.events.change_lane_reverse.send(Change::new(new_reverse, selection));
                    }
                });
            }
        } else {
            ui.add(
                Label::new("Nothing selected")
                .wrap(false)
            );
        }
    }
}
