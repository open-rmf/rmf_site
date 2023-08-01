/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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
    interaction::Hover,
    recency::ChangeRank,
    site::{
        Change, LayerVisibility, SiteID, VisibilityCycle, BeginEditDrawing,
        PreferredSemiTransparency,
    },
    widgets::{inspector::SelectionWidget, AppEvents, Icons, MoveLayerButton},
};
use bevy::prelude::*;
use bevy_egui::egui::{ImageButton, Ui, DragValue};

pub struct InspectLayer<'a, 'w, 's> {
    pub entity: Entity,
    pub icons: &'a Icons,
    /// Does the layer have a custom visibility setting?
    pub layer_vis: Option<LayerVisibility>,
    pub default_alpha: f32,
    // TODO(luca) make this an enum
    pub is_floor: bool,
    pub as_selected: bool,
    /// Outer Option: Can this be selected?
    /// Inner Option: Does this have a SiteID?
    pub site_id: Option<Option<SiteID>>,
    pub events: &'a mut AppEvents<'w, 's>,
}

impl<'a, 'w, 's> InspectLayer<'a, 'w, 's> {
    pub fn new(
        entity: Entity,
        icons: &'a Icons,
        events: &'a mut AppEvents<'w, 's>,
        layer_vis: Option<LayerVisibility>,
        default_alpha: f32,
        is_floor: bool,
    ) -> Self {
        Self {
            entity,
            icons,
            events,
            layer_vis,
            default_alpha,
            is_floor,
            as_selected: false,
            site_id: None,
        }
    }
    pub fn with_selecting(mut self, site_id: Option<SiteID>) -> Self {
        self.site_id = Some(site_id);
        self
    }

    pub fn as_selected(mut self, as_selected: bool) -> Self {
        self.as_selected = as_selected;
        self
    }

    pub fn show(self, ui: &mut Ui) {
        if let Some(site_id) = self.site_id {
            SelectionWidget::new(self.entity, site_id, self.icons, self.events)
                .as_selected(self.as_selected)
                .show(ui);

            if !self.is_floor {
                let response = ui.add(ImageButton::new(
                    self.events.layers.icons.edit.egui(), [18., 18.]
                )).on_hover_text("Edit Drawing");

                if response.hovered() {
                    self.events.request.hover.send(Hover(Some(self.entity)));
                }

                if response.clicked() {
                    self.events.layers.begin_edit_drawing.send(
                        BeginEditDrawing(self.entity)
                    );
                }
            }
        }

        let icon = self.icons.layer_visibility_of(self.layer_vis);
        let resp = ui
            .add(ImageButton::new(icon, [18., 18.]))
            .on_hover_text(format!(
                "Change to {}",
                self.layer_vis.next(self.default_alpha).label()
            ));
        if resp.hovered() {
            self.events.request.hover.send(Hover(Some(self.entity)));
        }
        if resp.clicked() {
            match self.layer_vis.next(self.default_alpha) {
                Some(v) => {
                    self.events
                        .layers
                        .layer_vis
                        .send(Change::new(v, self.entity).or_insert());
                }
                None => {
                    self.events
                        .commands
                        .entity(self.entity)
                        .remove::<LayerVisibility>();
                }
            }
        }

        if let Some(LayerVisibility::Alpha(mut alpha)) = self.layer_vis {
            if ui.add(
                DragValue::new(&mut alpha)
                    .clamp_range(0_f32..=1_f32)
                    .speed(0.01)
            ).changed() {
                self.events
                    .layers
                    .layer_vis
                    .send(Change::new(LayerVisibility::Alpha(alpha), self.entity));
                self.events
                    .layers
                    .preferred_alpha
                    .send(Change::new(PreferredSemiTransparency(alpha), self.entity));
            }
        }
    }
}
