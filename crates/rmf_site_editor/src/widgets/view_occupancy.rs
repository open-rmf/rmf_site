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

use crate::{occupancy::CalculateGrid, widgets::prelude::*, workspace::WorkspaceSaver, AppState};
use bevy::prelude::*;
use bevy_egui::egui::{CollapsingHeader, DragValue, Ui};
use rmf_site_egui::*;
use std::collections::HashSet;

/// Add a widget that provides a button for producing an occupancy grid
/// visualization.
#[derive(Default)]
pub struct ViewOccupancyPlugin {}

#[derive(Event)]
pub struct ExportOccupancy;

impl Plugin for ViewOccupancyPlugin {
    fn build(&self, app: &mut bevy::prelude::App) {
        app.init_resource::<OccupancyDisplay>()
            .add_event::<ExportOccupancy>()
            .add_plugins(PropertiesTilePlugin::<ViewOccupancy>::new())
            .add_systems(
                Update,
                handle_export_occupancy_menu.run_if(AppState::in_displaying_mode()),
            );
    }
}

fn handle_export_occupancy_menu(
    mut workspace_saver: WorkspaceSaver,
    mut export_event: EventReader<ExportOccupancy>,
) {
    for _ in export_event.read() {
        workspace_saver.export_occupancy_to_dialog();
    }
}

#[derive(SystemParam)]
pub struct ViewOccupancy<'w> {
    calculate_grid: EventWriter<'w, CalculateGrid>,
    export_grid: EventWriter<'w, ExportOccupancy>,
    display_occupancy: ResMut<'w, OccupancyDisplay>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w> WidgetSystem<Tile> for ViewOccupancy<'w> {
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

impl<'w> ViewOccupancy<'w> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            if ui.button("Calculate Occupancy").clicked() {
                self.calculate_grid.write(CalculateGrid {
                    cell_size: self.display_occupancy.cell_size,
                    floor: 0.01,
                    ceiling: 1.5,
                    ignore: HashSet::new(),
                });
            }
            if ui
                .add(
                    DragValue::new(&mut self.display_occupancy.cell_size)
                        .range(0.01..=f32::INFINITY)
                        .speed(0.01),
                )
                .changed()
            {
                if self.display_occupancy.cell_size > 0.1 {
                    self.calculate_grid.write(CalculateGrid {
                        cell_size: self.display_occupancy.cell_size,
                        floor: 0.01,
                        ceiling: 1.5,
                        ignore: HashSet::new(),
                    });
                }
            }
        });

        if ui.button("Export Occupancy").clicked() {
            /*self.calculate_grid.write(CalculateGrid {
                cell_size: self.display_occupancy.cell_size,
                floor: 0.01,
                ceiling: 1.5,
                ignore: HashSet::new(),
            });*/
            self.export_grid.write(ExportOccupancy);
            println!("Export occupancy clicked");
        }
    }
}

#[derive(Resource)]
pub struct OccupancyDisplay {
    pub cell_size: f32,
}

impl Default for OccupancyDisplay {
    fn default() -> Self {
        Self { cell_size: 0.1 }
    }
}
