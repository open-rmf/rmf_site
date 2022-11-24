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
    site::{CabinDoorId, LevelProperties, SiteID, ToggleLiftDoorAvailability},
    widgets::{
        inspector::{InspectOptionF32, SelectionWidget},
        AppEvents, Icons,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, DragValue, Ui};
use rmf_site_format::lift::*;

#[derive(SystemParam)]
pub struct InspectLiftParams<'w, 's> {
    pub cabins: Query<'w, 's, (&'static LiftCabin<Entity>, &'static RecallLiftCabin<Entity>)>,
    pub doors: Query<'w, 's, &'static LevelVisits<Entity>>,
    pub levels: Query<'w, 's, &'static LevelProperties>,
    pub icons: Res<'w, Icons>,
    pub site_id: Query<'w, 's, &'static SiteID>,
}

pub struct InspectLiftCabin<'a, 'w1, 's1, 'w2, 's2> {
    pub lift: Entity,
    pub params: &'a InspectLiftParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> InspectLiftCabin<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(
        lift: Entity,
        params: &'a InspectLiftParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            lift,
            params,
            events,
        }
    }

    pub fn show(mut self, ui: &mut Ui) -> Option<LiftCabin<Entity>> {
        let (cabin, recall) = match self.params.cabins.get(self.lift) {
            Ok(r) => r,
            Err(_) => return None,
        };
        let mut new_cabin = cabin.clone();
        match &mut new_cabin {
            LiftCabin::Rect(params) => {
                ui.horizontal(|ui| {
                    ui.label("width");
                    ui.add(
                        DragValue::new(&mut params.width)
                            .suffix("m")
                            .clamp_range(0.01..=std::f32::INFINITY)
                            .fixed_decimals(2)
                            .speed(0.01),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("depth");
                    ui.add(
                        DragValue::new(&mut params.depth)
                            .suffix("m")
                            .clamp_range(0.01..=std::f32::INFINITY)
                            .fixed_decimals(2)
                            .speed(0.01),
                    );
                });

                if let Some(new_t) = InspectOptionF32::new(
                    "Wall Thickness".to_string(),
                    params.wall_thickness,
                    recall
                        .wall_thickness
                        .unwrap_or(DEFAULT_CABIN_WALL_THICKNESS),
                )
                .clamp_range(0.001..=std::f32::INFINITY)
                .suffix("m".to_string())
                .min_decimals(2)
                .max_decimals(4)
                .speed(0.001)
                .show(ui)
                {
                    params.wall_thickness = new_t;
                }

                if let Some(new_gap) = InspectOptionF32::new(
                    "Gap".to_string(),
                    params.gap,
                    recall.gap.unwrap_or(DEFAULT_CABIN_GAP),
                )
                .clamp_range(0.001..=std::f32::INFINITY)
                .suffix("m".to_string())
                .min_decimals(2)
                .max_decimals(4)
                .speed(0.001)
                .show(ui)
                {
                    params.gap = new_gap;
                }

                if let Some(new_shift) = InspectOptionF32::new(
                    "Shift".to_string(),
                    params.shift,
                    recall.shift.unwrap_or(0.0),
                )
                .suffix("m".to_string())
                .min_decimals(2)
                .max_decimals(4)
                .speed(0.001)
                .show(ui)
                {
                    params.shift = new_shift;
                }

                let cabin_width = params.width;
                let cabin_gap = params.gap();
                for (face, placement) in params.doors_mut() {
                    if let Some(placement) = placement {
                        CollapsingHeader::new(format!("{} Door", face.label()))
                            .default_open(false)
                            .show(ui, |ui| {
                                SelectionWidget::new(
                                    placement.door,
                                    self.params.site_id.get(placement.door).copied().ok(),
                                    &self.params.icons,
                                    &mut self.events,
                                )
                                .show(ui);

                                ui.horizontal(|ui| {
                                    ui.label("width");
                                    ui.add(
                                        DragValue::new(&mut placement.width)
                                            .suffix("m")
                                            .clamp_range(0.001..=cabin_width - 0.001)
                                            .min_decimals(2)
                                            .max_decimals(4)
                                            .speed(0.005),
                                    );
                                });

                                if let Some(new_shift) = InspectOptionF32::new(
                                    "Shifted".to_string(),
                                    placement.shifted,
                                    0.0,
                                )
                                .suffix("m".to_string())
                                .min_decimals(2)
                                .max_decimals(4)
                                .speed(0.005)
                                .show(ui)
                                {
                                    placement.shifted = new_shift;
                                }

                                if let Some(new_gap) = InspectOptionF32::new(
                                    "Custom Gap".to_string(),
                                    placement.custom_gap,
                                    cabin_gap,
                                )
                                .suffix("m".to_string())
                                .clamp_range(0.0..=std::f32::INFINITY)
                                .min_decimals(2)
                                .max_decimals(4)
                                .speed(0.001)
                                .show(ui)
                                {
                                    placement.custom_gap = new_gap;
                                }

                                if let Some(new_t) = InspectOptionF32::new(
                                    "Thickness".to_string(),
                                    placement.thickness,
                                    DEFAULT_CABIN_DOOR_THICKNESS,
                                )
                                .suffix("m".to_string())
                                .clamp_range(0.001..=std::f32::INFINITY)
                                .min_decimals(2)
                                .max_decimals(4)
                                .speed(0.001)
                                .show(ui)
                                {
                                    placement.thickness = new_t;
                                }

                                if let Ok(visits) = self.params.doors.get(placement.door) {
                                    CollapsingHeader::new(format!("Level Access"))
                                        .default_open(true)
                                        .show(ui, |ui| {
                                            for level in &self.events.display.level.order {
                                                let mut visits_level = visits.contains(level);
                                                if ui
                                                    .checkbox(
                                                        &mut visits_level,
                                                        self.params
                                                            .levels
                                                            .get(*level)
                                                            .map(|n| &n.name)
                                                            .unwrap_or(&"<Unknown>".to_string()),
                                                    )
                                                    .changed()
                                                {
                                                    self.events.request.toggle_door_levels.send(
                                                        ToggleLiftDoorAvailability {
                                                            for_lift: self.lift,
                                                            on_level: *level,
                                                            cabin_door: CabinDoorId::RectFace(face),
                                                            door_available: visits_level,
                                                        },
                                                    );
                                                }
                                            }
                                        });
                                }
                            });
                    } else if let Some(current_level) = **self.events.request.current_level {
                        if ui.button(format!("Add {} Door", face.label())).clicked() {
                            self.events.request.toggle_door_levels.send(
                                ToggleLiftDoorAvailability {
                                    for_lift: self.lift,
                                    on_level: current_level,
                                    cabin_door: CabinDoorId::RectFace(face),
                                    door_available: true,
                                },
                            );
                        }
                    }
                }
            }
        }

        if new_cabin != *cabin {
            let (LiftCabin::Rect(new_params), LiftCabin::Rect(old_params)) =
                (&mut new_cabin, cabin);
            if new_params.width != old_params.width {
                if let Some(shift) = &mut new_params.shift {
                    // The user has already assigned a shift to the cabin,
                    // so any change to the width should be half-applied to
                    // the shift so that the two parameters aren't fighting
                    // against each other.
                    let delta = (new_params.width - old_params.width) / 2.0;
                    *shift -= delta;
                }
            }

            return Some(new_cabin);
        }

        return None;
    }
}
