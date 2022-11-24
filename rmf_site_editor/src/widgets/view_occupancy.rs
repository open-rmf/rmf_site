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

use crate::{occupancy::CalculateGrid, widgets::AppEvents};
use bevy_egui::egui::{DragValue, Ui};

pub struct OccupancyDisplay {
    pub cell_size: f32,
}

impl Default for OccupancyDisplay {
    fn default() -> Self {
        Self { cell_size: 0.5 }
    }
}

pub struct ViewOccupancy<'a, 'w2, 's2> {
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w2, 's2> ViewOccupancy<'a, 'w2, 's2> {
    pub fn new(events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { events }
    }

    pub fn show(self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Calculate Occupancy").clicked() {
                self.events.request.calculate_grid.send(CalculateGrid {
                    cell_size: self.events.display.occupancy.cell_size,
                    floor: 0.01,
                    ceiling: 1.5,
                });
            }
            if ui
                .add(
                    DragValue::new(&mut self.events.display.occupancy.cell_size)
                        .clamp_range(0.01..=f32::INFINITY)
                        .speed(0.01),
                )
                .changed()
            {
                if self.events.display.occupancy.cell_size > 0.1 {
                    self.events.request.calculate_grid.send(CalculateGrid {
                        cell_size: self.events.display.occupancy.cell_size,
                        floor: 0.01,
                        ceiling: 1.5,
                    });
                }
            }
        });
    }
}
