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
    mapf_rse::debug_panel::egui::DragValue,
    occupancy,
    occupancy::{CalculateGridRequest, OccupancyInfo},
    prelude::SystemState,
    site::Change,
};
use bevy::ecs::system::SystemParam;
use bevy_egui::egui::{
    self, Align, CollapsingHeader, Color32, ComboBox, Frame, Grid as EguiGrid, Response,
    ScrollArea, Stroke, Ui,
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
            .init_resource::<MAPFDebugDisplay>()
            .add_systems(Update, handle_debug_panel_visibility);
        let panel = PanelWidget::new(negotiation_debug_panel, &mut app.world_mut());
        let widget = Widget::new::<NegotiationDebugWidget>(&mut app.world_mut());
        app.world_mut().spawn((panel, widget));
    }
}

#[derive(Component, Debug, Clone)]
pub struct DebugGoal {
    pub location: String,
}

#[derive(SystemParam)]
pub struct NegotiationDebugWidget<'w, 's> {
    debugger_settings: ResMut<'w, DebuggerSettings>,
    negotiation_debug_data: ResMut<'w, NegotiationDebugData>,
    negotiation_params: ResMut<'w, NegotiationParams>,
    negotiation_request: EventWriter<'w, NegotiationRequest>,
    tasks: Query<'w, 's, &'static Task>,
    grids: Query<'w, 's, (Entity, &'static Grid)>,
    current_level: Res<'w, CurrentLevel>,
    child_of: Query<'w, 's, &'static ChildOf>,
    occupancy_info: ResMut<'w, OccupancyInfo>,
    calculate_grid: EventWriter<'w, CalculateGridRequest>,
    name_in_site: Query<'w, 's, &'static NameInSite>,
    open_sites: Query<'w, 's, Entity, With<NameOfSite>>,
    current_workspace: Res<'w, CurrentWorkspace>,
    mapf_info: Query<'w, 's, &'static MAPFDebugInfo>,
    commands: Commands<'w, 's>,
    path_visuals: Query<'w, 's, Entity, With<PathVisualMarker>>,
    display_mapf_debug: ResMut<'w, MAPFDebugDisplay>,
    robots_opose: Query<'w, 's, (Entity, Option<&'static Original<Pose>>), With<Robot>>,
    change_pose: EventWriter<'w, Change<Pose>>,
    locations: Query<'w, 's, &'static NameInSite, With<LocationTags>>,
    robots:
        Query<'w, 's, (Entity, &'static NameInSite, Option<&'static mut DebugGoal>), With<Robot>>,
}

fn negotiation_debug_panel(In(input): In<PanelWidgetInput>, world: &mut World) {
    if world.resource::<MAPFDebugDisplay>().show {
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
        params.show_robot_goals(ui);
        ui.separator();
        params.show_gotoplace_tasks(ui);
        ui.separator();
        params.show_occupancy_grid(ui);
        ui.separator();
        params.show_planner_settings(ui);
        ui.separator();
        params.show_generate_plan(ui);
        ui.separator();

        let Some(site) = params.current_workspace.to_site(&params.open_sites) else {
            return;
        };

        if let Some(debug_info) = params.mapf_info.get(site).ok() {
            match debug_info {
                MAPFDebugInfo::Success { .. } => {
                    params.show_successful_plan(ui);
                }
                MAPFDebugInfo::InProgress {
                    start_time,
                    task: _,
                } => {
                    Self::show_inprogress_plan(ui, start_time);
                }
                MAPFDebugInfo::Failed {
                    error_message,
                    entity_id_map,
                    negotiation_history,
                    conflicts,
                } => {
                    Self::show_failed_plan(
                        ui,
                        error_message,
                        entity_id_map,
                        negotiation_history,
                        conflicts,
                    );
                }
            }
        } else {
            ui.label("No planning started");
        }

        if ui.button("Close").clicked() {
            params.display_mapf_debug.show = false;
        }
    }
}

impl<'w, 's> NegotiationDebugWidget<'w, 's> {
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

    pub fn show_robot_goals(&mut self, ui: &mut Ui) {
        for (robot_entity, robot_name, robot_goal) in self.robots.iter_mut() {
            if let Some(mut goal) = robot_goal {
                ui.horizontal(|ui| {
                    ui.label(format!("{} goal: ", robot_name.0));

                    let selected_location_name = if goal.location.is_empty() {
                        "Selected location".into()
                    } else {
                        goal.location.clone()
                    };

                    let mut new_goal_location = goal.location.clone();

                    ComboBox::from_id_salt(format!("select_go_to_location_{}", robot_entity))
                        .selected_text(selected_location_name)
                        .show_ui(ui, |ui| {
                            for location_name in self.locations.iter() {
                                ui.selectable_value(
                                    &mut new_goal_location,
                                    location_name.0.clone(),
                                    location_name.0.clone(),
                                );
                            }
                        });

                    if goal.location != new_goal_location {
                        goal.location = new_goal_location;
                    }

                    if !goal.location.is_empty() {
                        if ui.button("Clear selection").clicked() {
                            goal.location = String::new();
                        }
                    }
                });
            } else {
                self.commands.entity(robot_entity).insert(DebugGoal {
                    location: "".into(),
                });
            }
        }
    }

    pub fn show_gotoplace_tasks(&mut self, ui: &mut Ui) {
        let tasks = self.tasks.iter().filter(|task| {
            if task.request().category() == GoToPlace::label() {
                true
            } else {
                false
            }
        });
        let mut num_tasks = 0;
        for task in tasks {
            ui.separator();
            ui.label(format!("Task {}", num_tasks));
            ui.label(format!("Robot name - {}", task.robot()));
            ui.label(format!("Description - {}", task.request().description()));
            num_tasks += 1;
        }
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
                self.calculate_grid.write(CalculateGridRequest);
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
        let mut allow_generate_plan = true;
        let mut error_msgs: Vec<&str> = Vec::new();

        if self.negotiation_params.queue_length_limit <= 0 {
            error_msgs.push("Set negotiation params queue length limit > 0");
            allow_generate_plan = false;
        }

        if let Some(site) = self.current_workspace.to_site(&self.open_sites) {
            if let Some(plan_info) = self.mapf_info.get(site).ok() {
                if matches!(*plan_info, MAPFDebugInfo::InProgress { .. }) {
                    error_msgs.push("Negotiation task is in progress");
                    allow_generate_plan = false;
                }
            }
        }

        let num_valid_robot_goals = self
            .robots
            .iter()
            .map(|(_, _, robot_goal)| {
                if let Some(debug_goal) = robot_goal {
                    if !debug_goal.location.is_empty() {
                        return true;
                    } else {
                        return false;
                    }
                } else {
                    return false;
                }
            })
            .filter(|is_valid| *is_valid)
            .count();
        if num_valid_robot_goals == 0 {
            error_msgs.push("No valid robot goals set");
            allow_generate_plan = false;
        }

        ui.add_enabled_ui(allow_generate_plan, |ui| {
            if ui.button("Generate Plans").clicked() {
                self.negotiation_request.write(NegotiationRequest);
            }
        });
        ui.end_row();
        if !error_msgs.is_empty() {
            ui.label("Unable to generate plan due to:");
        }
        ui.end_row();
        for err_msg in error_msgs {
            ui.label(format!("-{}", err_msg));
            ui.end_row();
        }
    }

    fn show_negotiation_history(negotiation_history: &Vec<NegotiationNode>, ui: &mut Ui) {
        CollapsingHeader::new("Negotiation history")
            .default_open(false)
            .show(ui, |ui| {
                let mut id_response_map = HashMap::<usize, &mut Response>::new();
                ScrollArea::vertical().show(ui, |ui| {
                    for negotiation_node in negotiation_history {
                        let _id = negotiation_node.id;
                        let _response =
                            show_negotiation_node(ui, &mut id_response_map, negotiation_node);
                        // id_response_map.insert(id, &mut response);
                    }
                });
            });
    }

    fn show_inprogress_plan(ui: &mut Ui, start_time: &Instant) {
        ui.label(format!(
            "Planning in Progress: {} s",
            start_time.elapsed().as_secs_f32()
        ));
    }

    fn show_failed_plan(
        ui: &mut Ui,
        error_message: &Option<String>,
        _entity_id_map: &HashMap<usize, Entity>,
        negotiation_history: &Vec<NegotiationNode>,
        _conflicts: &Vec<(Entity, Entity)>,
    ) {
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

        Self::show_negotiation_history(negotiation_history, ui);
    }

    pub fn show_successful_plan(&mut self, ui: &mut Ui) {
        let Some(site) = self.current_workspace.to_site(&self.open_sites) else {
            return;
        };

        let Some(plan_info) = self.mapf_info.get(site).ok() else {
            return;
        };

        let MAPFDebugInfo::Success {
            longest_plan_duration_s,
            elapsed_time,
            solution,
            entity_id_map,
            negotiation_history,
        } = plan_info
        else {
            return;
        };

        // Visualize
        ui.horizontal(|ui| {
            if ui.button("Clear Plans").clicked() {
                self.commands.entity(site).remove::<MAPFDebugInfo>();
                for e in self.path_visuals.iter() {
                    self.commands.entity(e).despawn();
                }
                // Reset back to start
                for (robot_entity, robot_opose) in self.robots_opose.iter() {
                    if let Some(opose) = robot_opose {
                        self.change_pose.write(Change::new(opose.0, robot_entity));
                    }
                    self.commands
                        .entity(robot_entity)
                        .remove::<Original<Pose>>();
                }
            }
        });

        for (i, proposal) in solution.proposals.iter().enumerate() {
            let Some(robot_entity) = entity_id_map.get(&proposal.0) else {
                warn!("Unable to find robot entity");
                continue;
            };
            ui.horizontal(|ui| {
                let text = if let Some(name) = self.name_in_site.get(*robot_entity).ok() {
                    format!("{} color: ", name.0)
                } else {
                    format!("robot {} color: ", i)
                };
                ui.label(text);
                egui::widgets::color_picker::color_edit_button_rgb(
                    ui,
                    &mut self.negotiation_debug_data.colors[i],
                );
            });
        }

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
                &mut self.debugger_settings.playback_speed,
                0.0..=8.0,
            ));
        });
        ui.end_row();

        if self.debugger_settings.playback_speed == 0.0 {
            if ui.button("Resume animation").clicked() {
                self.debugger_settings.playback_speed = 1.0;
            }
        } else {
            if ui.button("Pause animation").clicked() {
                self.debugger_settings.playback_speed = 0.0;
            }
        }
        ui.end_row();

        // Solution node
        ui.add_space(10.0);
        ui.label(format!(
            "Solution [found in {} s]",
            elapsed_time.as_secs_f32()
        ));

        show_negotiation_node(ui, &mut HashMap::new(), solution);

        Self::show_negotiation_history(negotiation_history, ui);
    }
}

fn show_negotiation_node(
    ui: &mut Ui,
    id_response_map: &mut HashMap<usize, &mut Response>,
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
                ui.label(format!("#{}", id));
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
    mut mapf_debug_window: ResMut<MAPFDebugDisplay>,
) {
    for event in menu_events.read() {
        if event.clicked() && event.source() == mapf_menu.debug_panel {
            mapf_debug_window.show = true;
        }
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct MAPFDebugDisplay {
    pub show: bool,
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
