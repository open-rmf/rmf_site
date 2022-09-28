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

use rmf_site_format::Angle;
use bevy::prelude::*;
use bevy_egui::egui::{
    Ui, DragValue,
};

pub struct InspectAngle<'a> {
    pub angle: &'a mut Angle,
}

impl<'a> InspectAngle<'a> {
    pub fn new(angle: &'a mut Angle) -> Self {
        Self{angle}
    }

    pub fn show(self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            match self.angle {
                Angle::Deg(deg) => {
                    ui.add(
                        DragValue::new(deg)
                        .min_decimals(0)
                        .max_decimals(1)
                        .speed(1.0)
                        .clamp_range(-180.0..=180.0)
                    );

                    let response = ui.button("deg")
                        .on_hover_text("Click to change to radians");

                    if response.clicked() {
                        *self.angle = Angle::Rad(self.angle.radians());
                    }
                },
                Angle::Rad(rad) => {
                    ui.add(
                        DragValue::new(rad)
                        .min_decimals(2)
                        .max_decimals(4)
                        .speed(std::f32::consts::PI/180.0)
                        .clamp_range(-std::f32::consts::PI..=std::f32::consts::PI)
                    );

                    let response = ui.button("rad")
                        .on_hover_text("Click to change to degrees");

                    if response.clicked() {
                        *self.angle = Angle::Deg(self.angle.degrees());
                    }
                },
            }
        });
    }
}
