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

use bevy_egui::egui::{DragValue, Ui};
use std::ops::RangeInclusive;

pub struct InspectOptionF32<'a> {
    title: &'a str,
    current_value: Option<f32>,
    assumed_value: f32,
    range: RangeInclusive<f32>,
    min_decimals: usize,
    max_decimals: Option<usize>,
    speed: f64,
    suffix: &'a str,
    tooltip: Option<&'a str>,
}

impl<'a> InspectOptionF32<'a> {
    pub fn new(title: &'a str, current_value: Option<f32>, assumed_value: f32) -> Self {
        Self {
            title,
            current_value,
            assumed_value,
            range: std::f32::NEG_INFINITY..=std::f32::INFINITY,
            min_decimals: 0,
            max_decimals: None,
            speed: 1.0,
            suffix: Default::default(),
            tooltip: Default::default(),
        }
    }

    pub fn clamp_range(mut self, range: RangeInclusive<f32>) -> Self {
        self.range = range;
        self
    }

    pub fn min_decimals(mut self, min_decimals: usize) -> Self {
        self.min_decimals = min_decimals;
        self
    }

    pub fn max_decimals(mut self, max_decimals: usize) -> Self {
        self.max_decimals = Some(max_decimals);
        self
    }

    pub fn speed(mut self, speed: f64) -> Self {
        self.speed = speed;
        self
    }

    pub fn suffix(mut self, suffix: &'a str) -> Self {
        self.suffix = suffix;
        self
    }

    pub fn tooltip(mut self, tooltip: &'a str) -> Self {
        self.tooltip = Some(tooltip);
        self
    }

    pub fn show(self, ui: &mut Ui) -> Option<Option<f32>> {
        ui.horizontal(|ui| {
            let mut has_value = self.current_value.is_some();
            let mut assumed_value = self.current_value.unwrap_or(self.assumed_value);
            ui.checkbox(&mut has_value, self.title);
            if has_value {
                let response = ui.add(
                    DragValue::new(&mut assumed_value)
                        .clamp_range(self.range)
                        .min_decimals(self.min_decimals)
                        .max_decimals_opt(self.max_decimals)
                        .speed(self.speed)
                        .suffix(self.suffix),
                );

                if let Some(tooltip) = self.tooltip {
                    response.on_hover_text(tooltip);
                }
            }

            if has_value {
                if self.current_value != Some(assumed_value) {
                    return Some(Some(assumed_value));
                }
            } else {
                if self.current_value.is_some() {
                    return Some(None);
                }
            }

            return None;
        })
        .inner
    }
}
