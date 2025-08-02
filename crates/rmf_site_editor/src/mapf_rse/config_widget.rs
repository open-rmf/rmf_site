/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use super::*;
use crate::{
    occupancy::{CalculateGrid, Grid},
    prelude::SystemState,
    site::{CurrentLevel, GoToPlace, Robot, Task, TaskKind},
    widgets::view_occupancy::OccupancyDisplay,
};
use bevy::{
    ecs::{hierarchy::ChildOf, system::SystemParam},
    prelude::*,
};
use bevy_egui::egui::{CollapsingHeader, DragValue, Grid as EguiGrid, Slider, Ui};
use rmf_site_egui::{Tile, WidgetSystem};

#[derive(SystemParam)]
pub struct MapfConfigWidget<'w, 's> {
    child_of: Query<'w, 's, &'static ChildOf>,
    current_level: Res<'w, CurrentLevel>,
    grids: Query<'w, 's, (Entity, &'static Grid)>,
    calculate_grid: EventWriter<'w, CalculateGrid>,
    negotiation_request: EventWriter<'w, NegotiationRequest>,
    negotiation_params: ResMut<'w, NegotiationParams>,
    negotiation_debug: ResMut<'w, NegotiationDebugData>,
    negotiation_task: Res<'w, NegotiationTask>,
    occupancy_display: ResMut<'w, OccupancyDisplay>,
    robots: Query<'w, 's, Entity, With<Robot>>,
    tasks: Query<'w, 's, &'static Task>,
}

impl<'w, 's> WidgetSystem<Tile> for MapfConfigWidget<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        ui.separator();

        CollapsingHeader::new("MAPF Configuration")
            .default_open(true)
            .show(ui, |ui| params.show_negotiation(ui));
    }
}

impl<'w, 's> MapfConfigWidget<'w, 's> {
    pub fn show_negotiation(&mut self, ui: &mut Ui) {
        // Visualize
        ui.horizontal(|ui| {
            ui.label("Visualize");
            ui.checkbox(
                &mut self.negotiation_debug.visualize_trajectories,
                "Trajectories",
            );
            ui.checkbox(&mut self.negotiation_debug.visualize_conflicts, "Conflicts");
            ui.checkbox(&mut self.negotiation_debug.visualize_keys, "Keys")
        });
        // Toggle debug panel
        ui.horizontal(|ui| {
            ui.label("Debug Panel");
            ui.checkbox(&mut self.negotiation_debug.show_debug_panel, "Enabled");
        });

        // Negotiation Request Properties
        // Agent tasks
        ui.separator();
        let num_tasks = self
            .tasks
            .iter()
            .filter(|task| {
                if task.request().category() == GoToPlace::label() {
                    true
                } else {
                    false
                }
            })
            .count();
        ui.label(format!("Tasks:    {}", num_tasks));
        // Grid Info
        let occupancy_grid = self
            .grids
            .iter()
            .filter_map(|(grid_entity, grid)| {
                if let Some(level_entity) = self.current_level.0 {
                    if self
                        .child_of
                        .get(grid_entity)
                        .is_ok_and(|co| co.parent() == level_entity)
                    {
                        Some(grid)
                    } else {
                        None
                    }
                } else {
                    None
                }
            })
            .next();
        ui.horizontal(|ui| {
            ui.label("Cell Size: ");
            // The button + slider combination help to indicate that cell size
            // requires initialization else grid is empty. These also differ
            // from those in the occupancy widget, as those do not ignore mobile
            // robots in calculation. However the cell size param used is
            // consistent, so any updated value will reflect accordingly
            if ui
                .add(
                    Slider::new(&mut self.occupancy_display.cell_size, 0.1..=1.0)
                        .suffix(" m")
                        .step_by(0.05),
                )
                .on_hover_text("Slide to calculate occupancy without robots")
                .changed()
            {
                self.calculate_grid.write(CalculateGrid {
                    cell_size: self.occupancy_display.cell_size,
                    ignore: self.robots.iter().collect(),
                    ..default()
                });
            }
        });
        ui.horizontal(|ui| {
            ui.label("Queue Length Limit: ");
            ui.add(
                DragValue::new(&mut self.negotiation_params.queue_length_limit)
                    .range(0..=std::usize::MAX)
                    .speed(1000),
            );
        });
        ui.label("Occupancy");
        ui.indent("occupancy_grid_info", |ui| {
            if let Some(grid) = occupancy_grid {
                EguiGrid::new("occupancy_map_info")
                    .num_columns(2)
                    .show(ui, |ui| {
                        let min_cell = grid.range.min_cell();
                        let max_cell = grid.range.max_cell();
                        ui.label("");
                        ui.label("|  x  |  y  |");
                        ui.end_row();
                        ui.label("Min");
                        ui.label(format!("|{:^5}|{:^5}|", min_cell.x, min_cell.y));
                        ui.end_row();
                        ui.label("Max");
                        ui.label(format!("|{:^5}|{:^5}|", max_cell.x, max_cell.y));
                        ui.end_row();
                        ui.label("Dimension");
                        ui.label(format!(
                            "{} x {}",
                            grid.range.max_cell().x - grid.range.min_cell().x,
                            grid.range.max_cell().y - grid.range.min_cell().y
                        ));
                        ui.end_row();
                    });
            } else {
                ui.label("None");
            }
        });
        // Generate Plan
        ui.horizontal(|ui| {
            let allow_generate_plan = num_tasks > 0
                && self.negotiation_params.queue_length_limit > 0
                && !self.negotiation_task.status.is_in_progress();

            ui.add_enabled_ui(allow_generate_plan, |ui| {
                if ui.button("Generate Plan").clicked() {
                    if occupancy_grid.is_none() {
                        self.calculate_grid.write(CalculateGrid {
                            cell_size: self.occupancy_display.cell_size,
                            ignore: self.robots.iter().collect(),
                            ..default()
                        });
                    }
                    self.negotiation_request.write(NegotiationRequest);
                }
            });
        });

        // Results
        ui.separator();
        match &self.negotiation_task.status {
            NegotiationTaskStatus::Complete {
                colors: _,
                elapsed_time,
                solution: _,
                negotiation_history,
                entity_id_map: _,
                error_message,
                conflicting_endpoints,
            } => {
                EguiGrid::new("negotiation_data")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Execution Time");
                        ui.label(format!("{:.2} s", elapsed_time.as_secs_f32()));
                        ui.end_row();
                        ui.label("Negotiation History");
                        ui.label(format!("{}", negotiation_history.len()));
                        ui.end_row();
                        ui.label("Endpoint Conflicts");
                        ui.label(format!("{}", conflicting_endpoints.len()));
                        ui.end_row();
                        ui.label("Error Message");
                        ui.label(error_message.clone().unwrap_or("None".to_string()));
                    });
            }
            NegotiationTaskStatus::InProgress { start_time } => {
                let elapsed_time = start_time.elapsed();
                ui.label(format!("In Progress: {}", elapsed_time.as_secs_f32()));
            }
            _ => {}
        }
    }
}
