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
    site::{
        CabinDoorId, Change, CurrentLevel, LevelElevation, NameInSite, ToggleLiftDoorAvailability,
    },
    widgets::{
        inspector::InspectOptionF32, prelude::*, Inspect, InspectionPlugin, LevelDisplay,
        SelectorWidget,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, DragValue, Ui};
use rmf_site_egui::WidgetSystem;
use rmf_site_format::lift::*;

#[derive(Default)]
pub struct InspectLiftPlugin {}

impl Plugin for InspectLiftPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LevelDisplay>()
            .add_plugins(InspectionPlugin::<InspectLiftCabin>::new());
    }
}

#[derive(SystemParam)]
pub struct InspectLiftCabin<'w, 's> {
    commands: Commands<'w, 's>,
    cabins: Query<'w, 's, (&'static LiftCabin<Entity>, &'static RecallLiftCabin<Entity>)>,
    doors: Query<'w, 's, &'static LevelVisits<Entity>>,
    levels: Query<'w, 's, (&'static NameInSite, &'static LevelElevation)>,
    display_level: Res<'w, LevelDisplay>,
    selector: SelectorWidget<'w, 's>,
    toggle_door_levels: EventWriter<'w, ToggleLiftDoorAvailability>,
    current_level: Res<'w, CurrentLevel>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectLiftCabin<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.show_widget(selection, ui);
    }
}

impl<'w, 's> InspectLiftCabin<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let Ok((cabin, recall)) = self.cabins.get(id) else {
            return;
        };
        let mut new_cabin = cabin.clone();
        match &mut new_cabin {
            LiftCabin::Rect(params) => {
                ui.horizontal(|ui| {
                    ui.label("width");
                    ui.add(
                        DragValue::new(&mut params.width)
                            .suffix("m")
                            .range(0.01..=std::f32::INFINITY)
                            .fixed_decimals(2)
                            .speed(0.01),
                    );
                });

                ui.horizontal(|ui| {
                    ui.label("depth");
                    ui.add(
                        DragValue::new(&mut params.depth)
                            .suffix("m")
                            .range(0.01..=std::f32::INFINITY)
                            .fixed_decimals(2)
                            .speed(0.01),
                    );
                });

                if let Some(new_t) = InspectOptionF32::new(
                    "Wall Thickness",
                    params.wall_thickness,
                    recall
                        .wall_thickness
                        .unwrap_or(DEFAULT_CABIN_WALL_THICKNESS),
                )
                .clamp_range(0.001..=std::f32::INFINITY)
                .suffix("m")
                .min_decimals(2)
                .max_decimals(4)
                .speed(0.001)
                .show(ui)
                {
                    params.wall_thickness = new_t;
                }

                if let Some(new_gap) = InspectOptionF32::new(
                    "Gap",
                    params.gap,
                    recall.gap.unwrap_or(DEFAULT_CABIN_GAP),
                )
                .clamp_range(0.001..=std::f32::INFINITY)
                .suffix("m")
                .min_decimals(2)
                .max_decimals(4)
                .speed(0.001)
                .show(ui)
                {
                    params.gap = new_gap;
                }

                if let Some(new_shift) =
                    InspectOptionF32::new("Shift", params.shift, recall.shift.unwrap_or(0.0))
                        .suffix("m")
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
                                self.selector.show_widget(placement.door, ui);
                                ui.horizontal(|ui| {
                                    ui.label("width");
                                    ui.add(
                                        DragValue::new(&mut placement.width)
                                            .suffix("m")
                                            .range(0.001..=cabin_width - 0.001)
                                            .min_decimals(2)
                                            .max_decimals(4)
                                            .speed(0.005),
                                    );
                                });

                                if let Some(new_shift) =
                                    InspectOptionF32::new("Shifted", placement.shifted, 0.0)
                                        .suffix("m")
                                        .min_decimals(2)
                                        .max_decimals(4)
                                        .speed(0.005)
                                        .show(ui)
                                {
                                    placement.shifted = new_shift;
                                }

                                if let Some(new_gap) = InspectOptionF32::new(
                                    "Custom Gap",
                                    placement.custom_gap,
                                    cabin_gap,
                                )
                                .suffix("m")
                                .clamp_range(0.0..=std::f32::INFINITY)
                                .min_decimals(2)
                                .max_decimals(4)
                                .speed(0.001)
                                .show(ui)
                                {
                                    placement.custom_gap = new_gap;
                                }

                                if let Some(new_t) = InspectOptionF32::new(
                                    "Thickness",
                                    placement.thickness,
                                    DEFAULT_CABIN_DOOR_THICKNESS,
                                )
                                .suffix("m")
                                .clamp_range(0.001..=std::f32::INFINITY)
                                .min_decimals(2)
                                .max_decimals(4)
                                .speed(0.001)
                                .show(ui)
                                {
                                    placement.thickness = new_t;
                                }

                                if let Ok(visits) = self.doors.get(placement.door) {
                                    CollapsingHeader::new(format!("Level Access"))
                                        .default_open(true)
                                        .show(ui, |ui| {
                                            for level in &self.display_level.order {
                                                let mut visits_level = visits.contains(level);
                                                if ui
                                                    .checkbox(
                                                        &mut visits_level,
                                                        self.levels
                                                            .get(*level)
                                                            .map(|(n, _)| &n.0)
                                                            .unwrap_or(&"<Unknown>".to_owned()),
                                                    )
                                                    .changed()
                                                {
                                                    self.toggle_door_levels.write(
                                                        ToggleLiftDoorAvailability {
                                                            for_lift: id,
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
                    } else if let Some(current_level) = **self.current_level {
                        if ui.button(format!("Add {} Door", face.label())).clicked() {
                            self.toggle_door_levels.write(ToggleLiftDoorAvailability {
                                for_lift: id,
                                on_level: current_level,
                                cabin_door: CabinDoorId::RectFace(face),
                                door_available: true,
                            });
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

            self.commands.trigger(Change::new(new_cabin, id));
        }
        ui.add_space(10.0);
    }
}
