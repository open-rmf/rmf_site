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
    site::*,
    interaction::Hover,
    widgets::{AppEvents, MoveLayer, Icons, inspector::SelectionWidget},
    recency::RecencyRanking,
};
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
};
use bevy_egui::egui::{ImageButton, Ui, CollapsingHeader};

#[derive(SystemParam)]
pub struct LayersParams<'w, 's> {
    pub floors: Query<'w, 's, &'static RecencyRanking<FloorMarker>>,
    pub drawings: Query<'w, 's, &'static RecencyRanking<DrawingMarker>>,
    pub floor_visibility: Query<'w, 's, &'static FloorVisibility>,
    pub site_id: Query<'w, 's, Option<&'static SiteID>>,
    pub icons: Res<'w, Icons>,
}

pub struct ViewLayers<'a, 'w1, 's1, 'w2, 's2> {
    params: &'a LayersParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> ViewLayers<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(
        params: &'a LayersParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
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

    fn show_rankings(
        &mut self,
        ranking: &Vec<Entity>,
        is_floor: bool,
        ui: &mut Ui,
    ) {
        ui.vertical(|ui| {
            for e in ranking {
                ui.horizontal(|ui| {
                    if is_floor {
                        let vis = self.params.floor_visibility.get(*e).ok().copied();
                        let icon = self.params.icons.floor_visibility_of(vis);
                        let resp = ui.add(ImageButton::new(icon, [18., 18.]))
                            .on_hover_text(format!("Change to {}", vis.next().label()));
                        if resp.hovered() {
                            self.events.request.hover.send(Hover(Some(*e)));
                        }
                        if resp.clicked() {
                            let new_vis = vis.next();
                            match new_vis {
                                Some(v) => {
                                    self.events.layers.change_floor_vis.send(
                                        Change::new(v, *e).or_insert()
                                    );
                                }
                                None => {
                                    self.events.commands.entity(*e).remove::<FloorVisibility>();
                                }
                            }
                        }
                    }

                    MoveLayer::up(
                        *e,
                        &mut self.events.layers.floors,
                        &self.params.icons,
                    )
                    .with_hover(&mut self.events.request.hover)
                    .show(ui);

                    MoveLayer::down(
                        *e,
                        &mut self.events.layers.floors,
                        &self.params.icons,
                    )
                    .with_hover(&mut self.events.request.hover)
                    .show(ui);

                    SelectionWidget::new(
                        *e,
                        self.params.site_id.get(*e).ok().flatten().copied(),
                        &self.params.icons,
                        &mut self.events,
                    )
                    .show(ui);
                });
            }
        });
    }
}
