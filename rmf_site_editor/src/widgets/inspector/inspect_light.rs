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
    site::{LightKind, RecallLightKind},
};
use bevy::prelude::*;
use bevy_egui::egui::{
    Ui, ComboBox, DragValue, Rgba,
    color_picker::{color_edit_button_rgba, Alpha},
};

pub struct InspectLightKind<'a> {
    pub kind: &'a LightKind,
    pub recall: &'a RecallLightKind,
}

impl<'a> InspectLightKind<'a> {
    pub fn new(kind: &'a LightKind, recall: &'a RecallLightKind) -> Self {
        Self { kind, recall }
    }

    pub fn show(self, ui: &mut Ui) -> Option<LightKind> {
        let mut new_kind = self.kind.clone();
        ui.horizontal(|ui| {
            ui.label("Light Kind:");
            ComboBox::from_id_source("Inspect Light Kind ComboBox")
                .selected_text(self.kind.label())
                .show_ui(ui, |ui| {
                    for variant in [
                        self.recall.assume_point(self.kind),
                        self.recall.assume_spot(self.kind),
                        self.recall.assume_directional(self.kind),
                    ] {
                        ui.selectable_value(&mut new_kind, variant.clone(), variant.label());
                    }
                });
        });

        match &mut new_kind {
            LightKind::Point(point) => {
                ui.horizontal(|ui| {
                    ui.label("Color");
                    color_edit(ui, &mut point.color);
                });
                ui.horizontal(|ui| {
                    ui.label("Intensity");
                    ui.add(
                        DragValue::new(&mut point.intensity)
                            .clamp_range(0_f32..=std::f32::INFINITY)
                            .speed(10)
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Range");
                    ui.add(
                        DragValue::new(&mut point.range)
                            .clamp_range(0_f32..=std::f32::INFINITY)
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Radius");
                    ui.add(
                        DragValue::new(&mut point.radius)
                            .clamp_range(0_f32..=std::f32::INFINITY)
                            .speed(0.1)
                    );
                });
                ui.checkbox(&mut point.enable_shadows, "Enable Shadows");
            }
            LightKind::Spot(spot) => {
                ui.horizontal(|ui| {
                    ui.label("Color");
                    color_edit(ui, &mut spot.color);
                });
                ui.horizontal(|ui| {
                    ui.label("Intensity");
                    ui.add(
                        DragValue::new(&mut spot.intensity)
                            .clamp_range(0_f32..=std::f32::INFINITY)
                            .speed(10)
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Range");
                    ui.add(
                        DragValue::new(&mut spot.range)
                            .clamp_range(0_f32..=std::f32::INFINITY)
                    );
                });
                ui.horizontal(|ui| {
                    ui.label("Radius");
                    ui.add(
                        DragValue::new(&mut spot.radius)
                            .clamp_range(0_f32..=std::f32::INFINITY)
                            .speed(0.1)
                    );
                });
                ui.checkbox(&mut spot.enable_shadows, "Enable Shadows");
            }
            LightKind::Directional(dir) => {
                ui.horizontal(|ui| {
                    ui.label("Color");
                    color_edit(ui, &mut dir.color);
                });
                ui.horizontal(|ui| {
                    ui.label("Illuminance");
                    ui.add(
                        DragValue::new(&mut dir.illuminance)
                            .clamp_range(0_f32..=std::f32::INFINITY)
                            .speed(1000)
                    );
                });
                ui.checkbox(&mut dir.enable_shadows, "Enable Shadows");
            }
        }

        if new_kind != *self.kind {
            return Some(new_kind);
        }

        None
    }
}

fn color_edit(ui: &mut Ui, color: &mut [f32; 4]) {
    let mut rgba = Rgba::from_rgba_premultiplied(
        color[0], color[1], color[2], color[3]
    );
    color_edit_button_rgba(ui, &mut rgba, Alpha::OnlyBlend);
    *color = [rgba.r(), rgba.g(), rgba.b(), rgba.a()];
}
