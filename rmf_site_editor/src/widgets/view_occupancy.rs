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

use crate::{AppState, occupancy::CalculateGrid, widgets::prelude::*};
use bevy::prelude::*;
use bevy_egui::egui::{DragValue, CollapsingHeader, Ui};

#[derive(Default)]
pub struct ViewOccupancyPlugin {

}

impl Plugin for ViewOccupancyPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<OccupancyDisplay>();
        let widget = Widget::new::<ExViewOccupancy>(&mut app.world);
        let properties_panel = app.world.resource::<PropertiesPanel>().id;
        app.world.spawn(widget).set_parent(properties_panel);
    }
}

#[derive(SystemParam)]
pub struct ExViewOccupancy<'w> {
    calculate_grid: EventWriter<'w, CalculateGrid>,
    display_occupancy: ResMut<'w, OccupancyDisplay>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w> WidgetSystem<Tile> for ExViewOccupancy<'w> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if *params.app_state.get() != AppState::SiteEditor {
            return;
        }
        CollapsingHeader::new("Occupancy")
            .default_open(false)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w> ExViewOccupancy<'w> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Calculate Occupancy").clicked() {
                self.calculate_grid.send(CalculateGrid {
                    cell_size: self.display_occupancy.cell_size,
                    floor: 0.01,
                    ceiling: 1.5,
                });
            }
            if ui
                .add(
                    DragValue::new(&mut self.display_occupancy.cell_size)
                        .clamp_range(0.01..=f32::INFINITY)
                        .speed(0.01),
                )
                .changed()
            {
                if self.display_occupancy.cell_size > 0.1 {
                    self.calculate_grid.send(CalculateGrid {
                        cell_size: self.display_occupancy.cell_size,
                        floor: 0.01,
                        ceiling: 1.5,
                    });
                }
            }
        });
    }
}

#[derive(Resource)]
pub struct OccupancyDisplay {
    pub cell_size: f32,
}

impl Default for OccupancyDisplay {
    fn default() -> Self {
        Self { cell_size: 0.5 }
    }
}
