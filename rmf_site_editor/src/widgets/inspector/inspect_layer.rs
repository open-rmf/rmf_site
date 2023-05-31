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
    site::{Change, Cycle, LayerVisibility, SiteID},
    widgets::{inspector::SelectionWidget, AppEvents, Icons, MoveLayer},
};
use bevy::prelude::*;
use bevy_egui::egui::{ImageButton, Ui};

pub struct InspectLayer<'a, 'w, 's> {
    pub entity: Entity,
    pub icons: &'a Icons,
    /// Does the floor have a custom visibility setting?
    pub layer_vis: Option<LayerVisibility>,
    // TODO(luca) make this an enum
    pub is_floor: bool,
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
        is_floor: bool,
    ) -> Self {
        Self {
            entity,
            icons,
            events,
            layer_vis,
            is_floor,
            site_id: None,
        }
    }
    pub fn with_selecting(mut self, site_id: Option<SiteID>) -> Self {
        self.site_id = Some(site_id);
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let icon = self.icons.layer_visibility_of(self.layer_vis);
        let resp = ui
            .add(ImageButton::new(icon, [18., 18.]))
            .on_hover_text(format!("Change to {}", self.layer_vis.next().label()));
        if resp.hovered() {
            self.events.request.hover.send(Hover(Some(self.entity)));
        }
        if resp.clicked() {
            match self.layer_vis.next() {
                Some(v) => {
                    self.events
                        .layers
                        .change_layer_vis
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

        if self.is_floor {
            Self::move_layers(
                self.entity,
                &self.icons,
                &mut self.events.layers.floors,
                &mut self.events.request.hover,
                ui,
            );
        } else {
            Self::move_layers(
                self.entity,
                &self.icons,
                &mut self.events.layers.drawings,
                &mut self.events.request.hover,
                ui,
            );
        };

        if let Some(site_id) = self.site_id {
            SelectionWidget::new(self.entity, site_id, self.icons, self.events).show(ui);
        }
    }

    fn move_layers<T: Component>(
        entity: Entity,
        icons: &Icons,
        mover: &mut EventWriter<'w, 's, ChangeRank<T>>,
        hover: &mut ResMut<'w, Events<Hover>>,
        ui: &mut Ui,
    ) {
        MoveLayer::to_top(entity, mover, icons)
            .with_hover(hover)
            .show(ui);

        MoveLayer::up(entity, mover, icons)
            .with_hover(hover)
            .show(ui);

        MoveLayer::down(entity, mover, icons)
            .with_hover(hover)
            .show(ui);

        MoveLayer::to_bottom(entity, mover, icons)
            .with_hover(hover)
            .show(ui);
    }
}
