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
    interaction::Selection,
    recency::RecencyRanking,
    site::*,
    widgets::{inspector::InspectLayer, AppEvents, Icons},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, CollapsingHeader, Ui};

#[derive(SystemParam)]
pub struct LayersParams<'w, 's> {
    pub floors: Query<'w, 's, &'static RecencyRanking<FloorMarker>>,
    pub drawings: Query<'w, 's, &'static RecencyRanking<DrawingMarker>>,
    pub floor_visibility: Query<'w, 's, &'static FloorVisibility>,
    pub site_id: Query<'w, 's, Option<&'static SiteID>>,
    pub icons: Res<'w, Icons>,
    pub selection: Res<'w, Selection>,
}

pub struct ViewLayers<'a, 'w1, 's1, 'w2, 's2> {
    params: &'a LayersParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> ViewLayers<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(params: &'a LayersParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { params, events }
    }

    pub fn show(mut self, ui: &mut Ui) {
        let current_level = match &self.events.request.current_level.0 {
            Some(s) => *s,
            None => return,
        };

        if let Ok(ranking) = self.params.floors.get(current_level) {
            CollapsingHeader::new("Floors")
                .default_open(true)
                .show(ui, |ui| {
                    ui.horizontal(|ui| {
                        let vis = *self.events.layers.global_floor_vis;
                        let icon = self.params.icons.floor_visibility_of(Some(vis));
                        let resp = ui
                            .add(Button::image_and_text(icon, [18., 18.], "Global"))
                            .on_hover_text(format!("Change to {}", vis.next().label()));
                        if resp.clicked() {
                            *self.events.layers.global_floor_vis = vis.next();
                        }
                    });
                    self.show_rankings(ranking.entities(), true, ui);
                });
        }

        if let Ok(ranking) = self.params.drawings.get(current_level) {
            CollapsingHeader::new("Drawings")
                .default_open(true)
                .show(ui, |ui| {
                    self.show_rankings(ranking, false, ui);
                });
        }
    }

    fn show_rankings(&mut self, ranking: &Vec<Entity>, is_floor: bool, ui: &mut Ui) {
        ui.vertical(|ui| {
            for e in ranking.iter().rev() {
                ui.horizontal(|ui| {
                    let mut layer = InspectLayer::new(*e, &self.params.icons, &mut self.events)
                        .with_selecting(self.params.site_id.get(*e).ok().flatten().copied());

                    if is_floor {
                        layer = layer.as_floor(self.params.floor_visibility.get(*e).ok().copied());
                    }

                    layer.show(ui);

                    if Some(*e) == self.params.selection.0 {
                        ui.label("Selected");
                    }
                });
            }
        });
    }
}
