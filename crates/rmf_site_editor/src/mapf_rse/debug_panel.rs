/*
 * Copyright (C) 2024 active Source Robotics Foundation
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
    mapf_rse::debug_panel::egui::DragValue, occupancy, occupancy::CalculateGrid,
    prelude::SystemState,
};
use bevy::ecs::system::SystemParam;
use bevy_egui::egui::{
    self, Align, CollapsingHeader, Color32, Frame, Grid as EguiGrid, Response, ScrollArea, Stroke,
    Ui,
};
use rmf_site_egui::{
    MenuEvent, MenuItem, PanelWidget, PanelWidgetInput, ToolMenu, TryShowWidgetWorld, Widget,
    WidgetSystem,
};
use rmf_site_format::{Task, TaskKind};

#[derive(Default)]
pub struct NegotiationDebugPlugin;

impl Plugin for NegotiationDebugPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NegotiationDebugData>()
            .init_resource::<MAPFMenu>()
            .init_resource::<OccupancyInfo>()
            .add_systems(Update, handle_debug_panel_visibility);
        let panel = PanelWidget::new(negotiation_debug_panel, &mut app.world_mut());
        let widget = Widget::new::<NegotiationDebugWidget>(&mut app.world_mut());
        app.world_mut().spawn((panel, widget));
    }
}

#[derive(Resource)]
pub struct OccupancyInfo {
    cell_size: f32,
}

impl Default for OccupancyInfo {
    fn default() -> OccupancyInfo {
        OccupancyInfo { cell_size: 0.1 }
    }
}

#[derive(SystemParam)]
pub struct NegotiationDebugWidget<'w, 's> {
    negotiation_task: ResMut<'w, NegotiationTask>,
    negotiation_debug_data: ResMut<'w, NegotiationDebugData>,
    negotiation_params: ResMut<'w, NegotiationParams>,
    negotiation_request: EventWriter<'w, NegotiationRequest>,
    tasks: Query<'w, 's, &'static Task>,
    grids: Query<'w, 's, (Entity, &'static Grid)>,
    current_level: Res<'w, CurrentLevel>,
    child_of: Query<'w, 's, &'static ChildOf>,
    occupancy_info: ResMut<'w, OccupancyInfo>,
    calculate_grid: EventWriter<'w, CalculateGrid>,
    robots: Query<'w, 's, Entity, With<Robot>>,
    open_sites: Query<'w, 's, Entity, With<NameOfSite>>,
    current_workspace: Res<'w, CurrentWorkspace>,
    mapf_info: Query<'w, 's, &'static MAPFDebugInfo>,
    commands: Commands<'w, 's>,
    path_visuals: Query<'w, 's, Entity, With<PathVisualMarker>>,
}

fn negotiation_debug_panel(In(input): In<PanelWidgetInput>, world: &mut World) {
    if world.resource::<NegotiationDebugData>().show_debug_panel {
        egui::SidePanel::left("negotiation_debug_panel")
            .resizable(true)
            .min_width(320.0)
            .show(&input.context, |ui| {
                if let Err(err) = world.try_show(input.id, ui) {
                    error!("Unable to display debug panel: {err:?}");
                }
            });
    }
}

impl<'w, 's> WidgetSystem for NegotiationDebugWidget<'w, 's> {
    fn show(_: (), ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);

        ui.heading("Negotiation Debugger");
        params.show_gotoplace_tasks(ui);
        ui.separator();
        params.show_occupancy_grid(ui);
        ui.separator();
        params.show_planner_settings(ui);
        ui.separator();
        params.show_generate_plan(ui);
        ui.separator();

        match params.negotiation_task.status {
            NegotiationTaskStatus::Complete { .. } => {
                params.show_completed(ui);
            }
            NegotiationTaskStatus::InProgress { start_time } => {
                ui.label(format!(
                    "Planning in Progress: {} s",
                    start_time.elapsed().as_secs_f32()
                ));
            }
            _ => {
                ui.label("No negotiation started");
            }
        }
    }
}

impl<'w, 's> NegotiationDebugWidget<'w, 's> {
    fn get_gotoplace_tasks(&self) -> usize {
        self.tasks
            .iter()
            .filter(|task| {
                if task.request().category() == GoToPlace::label() {
                    true
                } else {
                    false
                }
            })
            .count()
    }

    fn get_occupancy_grid(&self) -> std::option::Option<&occupancy::Grid> {
        // Occupancy Grid Info
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
        occupancy_grid
    }

    pub fn show_gotoplace_tasks(&mut self, ui: &mut Ui) {
        // Negotiation Request Properties
        // Agent tasks
        ui.separator();
        ui.label(format!(
            "# of Robot GoToPlace Tasks: {}",
            self.get_gotoplace_tasks()
        ));
    }

    pub fn show_occupancy_grid(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            ui.label("Cell Size: ");
            // The button + slider combination help to indicate that cell size
            // requires initialization else grid is empty. These also differ
            // from those in the occupancy widget, as those do not ignore mobile
            // robots in calculation. However the cell size param used is
            // consistent, so any updated value will reflect accordingly
            ui.add(
                DragValue::new(&mut self.occupancy_info.cell_size)
                    .range(0.1..=1.0)
                    .suffix(" m")
                    .speed(0.01),
            )
            .on_hover_text("Slide to calculate occupancy without robots");
            if ui
                .button("Calculate Occupancy")
                .on_hover_text("Click to calculate occupancy without robots")
                .clicked()
            {
                self.calculate_grid.write(CalculateGrid {
                    cell_size: self.occupancy_info.cell_size,
                    ignore: self.robots.iter().collect(),
                    ..default()
                });
            }
        });

        let occupancy_grid = self.get_occupancy_grid();

        ui.label("Occupancy");
        ui.indent("occupancy_grid_info", |ui| {
            if let Some(grid) = occupancy_grid {
                EguiGrid::new("occupancy_map_info")
                    .num_columns(2)
                    .show(ui, |ui| {
                        ui.label("Min Cell");
                        ui.label(format!("{:?}", grid.range.min_cell()));
                        ui.end_row();
                        ui.label("Max Cell");
                        ui.label(format!("{:?}", grid.range.max_cell()));
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
    }

    pub fn show_planner_settings(&mut self, ui: &mut Ui) {
        // Planner settings
        ui.horizontal(|ui| {
            ui.label("Queue Length Limit: ");
            ui.add(
                DragValue::new(&mut self.negotiation_params.queue_length_limit)
                    .range(0..=std::usize::MAX)
                    .speed(1000),
            );
        });
        ui.end_row();
    }

    pub fn show_generate_plan(&mut self, ui: &mut Ui) {
        ui.horizontal(|ui| {
            let mut allow_generate_plan = true;

            let mut error_msg = "Test".to_owned();

            if self.negotiation_params.queue_length_limit <= 0 {
                error_msg += "Set negotiation params queue length limit > 0\n";
                allow_generate_plan = false;
            }

            if self.negotiation_task.status.is_in_progress() {
                error_msg += "Negotiation task is in progress\n";
                allow_generate_plan = false;
            }

            let num_gotoplace_tasks = self.get_gotoplace_tasks();
            if num_gotoplace_tasks <= 0 {
                error_msg += "No gotoplace tasks\n";
                allow_generate_plan = false;
            }

            ui.add_enabled_ui(allow_generate_plan, |ui| {
                if ui
                    .button("Generate Plans")
                    .on_hover_text(error_msg)
                    .clicked()
                {
                    let occupancy_grid = self.get_occupancy_grid();
                    if occupancy_grid.is_none() {
                        self.calculate_grid.write(CalculateGrid {
                            cell_size: self.occupancy_info.cell_size,
                            ignore: self.robots.iter().collect(),
                            ..default()
                        });
                    }
                    self.negotiation_request.write(NegotiationRequest);
                }
            });
        });
    }

    fn show_negotiation_history(
        mut negotiation_debug_data: &mut ResMut<NegotiationDebugData>,
        negotiation_history: &Vec<NegotiationNode>,
        ui: &mut Ui,
    ) {
        CollapsingHeader::new("Negotiation history")
            .default_open(false)
            .show(ui, |ui| {
                let mut id_response_map = HashMap::<usize, &mut Response>::new();
                ScrollArea::vertical().show(ui, |ui| {
                    for negotiation_node in negotiation_history {
                        let _id = negotiation_node.id;
                        let _response = show_negotiation_node(
                            ui,
                            &mut id_response_map,
                            &mut negotiation_debug_data,
                            negotiation_node,
                        );
                        // id_response_map.insert(id, &mut response);
                    }
                });
            });
    }

    fn show_failed_plan(
        mut negotiation_debug_data: &mut ResMut<NegotiationDebugData>,
        plan_info: &MAPFDebugInfo,
        ui: &mut Ui,
    ) {
        if let MAPFDebugInfo::Failed {
            error_message,
            entity_id_map: _,
            negotiation_history,
            conflicts: _,
        } = plan_info
        {
            // Error display
            ui.add_space(10.0);
            ui.label("Errors");
            if let Some(error_message) = error_message {
                outline_frame(ui, |ui| {
                    ui.horizontal_wrapped(|ui| {
                        ui.label(error_message.clone());
                    });
                });
            } else {
                outline_frame(ui, |ui| {
                    ui.label("No errors");
                });
            }

            Self::show_negotiation_history(&mut negotiation_debug_data, negotiation_history, ui);
        }
    }

    pub fn show_completed(&mut self, ui: &mut Ui) {
        let Some(site) = self.current_workspace.to_site(&self.open_sites) else {
            return;
        };

        let Some(plan_info) = self.mapf_info.get(site).ok() else {
            return;
        };

        let MAPFDebugInfo::Success {
            longest_plan_duration_s,
            colors: _,
            elapsed_time,
            solution,
            entity_id_map: _,
            path_mesh_info: _,
            negotiation_history,
        } = plan_info
        else {
            Self::show_failed_plan(&mut self.negotiation_debug_data, plan_info, ui);
            return;
        };

        // Visualize
        ui.horizontal(|ui| {
            if ui.button("Clear Plans").clicked() {
                self.commands.entity(site).remove::<MAPFDebugInfo>();
                self.negotiation_task.reset();
                for e in self.path_visuals.iter() {
                    self.commands.entity(e).despawn();
                }
                self.negotiation_debug_data.reset();
            }
        });

        ui.horizontal(|ui| {
            ui.label("Plan time: ");
            ui.add(egui::Slider::new(
                &mut self.negotiation_debug_data.time,
                0.0..=*longest_plan_duration_s,
            ));
        });
        ui.end_row();

        ui.horizontal(|ui| {
            ui.label("Playback speed: ");
            ui.add(egui::Slider::new(
                &mut self.negotiation_debug_data.playback_speed,
                0.0..=8.0,
            ));
        });
        ui.end_row();

        if self.negotiation_debug_data.playback_speed == 0.0 {
            if ui.button("Resume animation").clicked() {
                self.negotiation_debug_data.playback_speed = 1.0;
            }
        } else {
            if ui.button("Pause animation").clicked() {
                self.negotiation_debug_data.playback_speed = 0.0;
            }
        }
        ui.end_row();

        // Solution node
        ui.add_space(10.0);
        ui.label(format!(
            "Solution [found in {} s]",
            elapsed_time.as_secs_f32()
        ));

        show_negotiation_node(
            ui,
            &mut HashMap::new(),
            &mut self.negotiation_debug_data,
            solution,
        );

        Self::show_negotiation_history(&mut self.negotiation_debug_data, negotiation_history, ui);
    }
}

fn show_negotiation_node(
    ui: &mut Ui,
    id_response_map: &mut HashMap<usize, &mut Response>,
    negotiation_debug_data: &mut ResMut<NegotiationDebugData>,
    node: &NegotiationNode,
) -> Response {
    Frame::default()
        .inner_margin(4.0)
        .fill(Color32::DARK_GRAY)
        .corner_radius(2.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());

            let id = node.id;
            ui.horizontal(|ui| {
                let selected = negotiation_debug_data.selected_negotiation_node == Some(id);
                if ui.radio(selected, format!("#{}", node.id)).clicked() {
                    negotiation_debug_data.selected_negotiation_node = Some(id);
                }
                ui.label("|");
                ui.label(format!("Keys: {}", node.keys.len()));
                ui.label("|");
                ui.label(format!("Conflicts: {}", node.negotiation.conflicts.len()));
                ui.label("|");
                ui.label("Parent");
                match node.parent {
                    Some(parent) => {
                        if ui.button(format!("#{}", parent)).clicked() {
                            if let Some(response) = id_response_map.get_mut(&parent) {
                                response.scroll_to_me(Some(Align::Center));
                            }
                        }
                    }
                    None => {
                        ui.label("None");
                    }
                }
            });

            CollapsingHeader::new("Information")
                .id_salt(id.to_string() + "node_info")
                .default_open(false)
                .show(ui, |ui| {
                    ui.label("Keys");
                    for key in &node.keys {
                        ui.label(format!("{:?}", key));
                    }
                });
        })
        .response
}

fn outline_frame<R>(ui: &mut Ui, add_body: impl FnOnce(&mut Ui) -> R) -> Response {
    Frame::default()
        .inner_margin(4.0)
        .stroke(Stroke::new(1.0, Color32::GRAY))
        .corner_radius(2.0)
        .show(ui, |ui| {
            ui.set_width(ui.available_width());
            ui.add_enabled_ui(true, add_body);
        })
        .response
}

fn handle_debug_panel_visibility(
    mut menu_events: EventReader<MenuEvent>,
    mapf_menu: Res<MAPFMenu>,
    mut negotiation_debug: ResMut<NegotiationDebugData>,
) {
    for event in menu_events.read() {
        if event.clicked() && event.source() == mapf_menu.debug_panel {
            negotiation_debug.show_debug_panel = true;
        }
    }
}

#[derive(Resource)]
pub struct MAPFMenu {
    debug_panel: Entity,
}

impl FromWorld for MAPFMenu {
    fn from_world(world: &mut World) -> Self {
        // Tools menu
        let tool_header = world.resource::<ToolMenu>().get();
        let debug_panel = world
            .spawn(MenuItem::Text("Debug Panel".into()))
            .insert(ChildOf(tool_header))
            .id();

        MAPFMenu { debug_panel }
    }
}
