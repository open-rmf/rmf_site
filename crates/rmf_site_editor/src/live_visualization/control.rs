/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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

use bevy::ecs::system::{SystemParam, SystemState};
use bevy::prelude::*;
use bevy_egui::egui::{self, Color32};
use rmf_site_egui::{
    PropertiesPanel, PropertiesTilePlugin, ShowInStream, TabGroup, Tile, WidgetSystem,
};
use rmf_site_format::{Group, NameInSite, Pending, Pose, Robot};
use std::collections::HashMap;

use super::live_state::{LiveStreamState, LoadSiteStatus};
use super::planned_paths::LivePathsState;

const PENDING_COLOR: Color32 = Color32::from_rgb(250, 255, 94);
const MOVING_COLOR: Color32 = Color32::from_rgb(41, 224, 255);
const REACHED_COLOR: Color32 = Color32::from_rgb(69, 255, 94);

#[derive(Default)]
pub struct RobotPathControlPlugin {}

impl Plugin for RobotPathControlPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<RobotPathControlState>()
            .add_systems(
                Update,
                update_robot_path_control_ui
                    .run_if(in_state(crate::AppState::SiteStream))
                    .run_if(LiveStreamState::in_connected_mode()),
            )
            .add_systems(
                OnExit(crate::AppState::SiteStream),
                cleanup_robot_path_control,
            )
            .add_systems(
                PreUpdate,
                cleanup_robot_path_control
                    .run_if(in_state(crate::AppState::SiteStream))
                    .run_if(not(LiveStreamState::in_connected_mode())),
            );

        if app.world().get_resource::<PropertiesPanel>().is_some() {
            app.add_plugins(
                PropertiesTilePlugin::<RobotPathControlWidget>::new("Fleet")
                    .in_group(TabGroup::Top)
                    .show_in_stream(ShowInStream::Interactive),
            );
        }
    }
}

fn cleanup_robot_path_control(mut control_state: ResMut<RobotPathControlState>) {
    if !control_state.is_empty() {
        *control_state = RobotPathControlState::default();
    }
}

#[derive(Clone, Debug)]
struct RobotDestination {
    position: Vec2,
    location_name: Option<String>,
}

impl RobotDestination {
    fn display_label(&self) -> String {
        self.location_name
            .clone()
            .unwrap_or_else(|| format!("({:.2}, {:.2})", self.position.x, self.position.y))
    }
}

#[derive(Default, Resource)]
struct RobotPathControlState {
    active_routing_robot: Option<String>,
    current_destinations: HashMap<String, RobotDestination>,
    selected_destinations: HashMap<String, RobotDestination>,
}

impl RobotPathControlState {
    fn is_empty(&self) -> bool {
        self.active_routing_robot.is_none()
            && self.current_destinations.is_empty()
            && self.selected_destinations.is_empty()
    }
}

fn update_robot_path_control_ui(
    mut control_state: ResMut<RobotPathControlState>,
    path_state: Res<LivePathsState>,
) {
    for (robot_name, path_data) in &path_state.0 {
        if let Some(final_pos) = path_data.final_waypoint() {
            control_state
                .current_destinations
                .entry(robot_name.clone())
                .and_modify(|dest| {
                    dest.position = final_pos;
                })
                .or_insert(RobotDestination {
                    position: final_pos,
                    location_name: None,
                });
        }
    }
}

#[derive(SystemParam)]
struct RobotPathControlWidget<'w, 's> {
    app_state: Res<'w, State<crate::AppState>>,
    livestream_state: Res<'w, LiveStreamState>,
    control_state: ResMut<'w, RobotPathControlState>,
    path_state: Res<'w, LivePathsState>,
    robots: Query<
        'w,
        's,
        (&'static NameInSite, &'static Pose),
        (With<Robot>, Without<Group>, Without<Pending>),
    >,
}

impl<'w, 's> WidgetSystem<Tile> for RobotPathControlWidget<'w, 's> {
    fn show(_: Tile, ui: &mut egui::Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if *params.app_state.get() != crate::AppState::SiteStream {
            return;
        }
        params.show_widget(ui);
    }
}

impl<'w, 's> RobotPathControlWidget<'w, 's> {
    fn show_widget(&mut self, ui: &mut egui::Ui) {
        if !self.livestream_state.site_loaded
            || self.livestream_state.load_site_status != LoadSiteStatus::None
        {
            // If site has not finished loading, show loading text
            ui.vertical_centered(|ui| {
                ui.add_space(20.0);
                ui.label(
                    egui::RichText::new("Pending site load...")
                        .italics()
                        .color(Color32::from_gray(160)),
                );
            });
            return;
        }
        if !self.livestream_state.is_connected() {
            return;
        }

        let mut robot_list: Vec<(String, Vec2)> = self
            .robots
            .iter()
            .map(|(name, pose)| (name.0.clone(), Vec2::new(pose.trans[0], pose.trans[1])))
            .collect();
        robot_list.sort_by(|a, b| a.0.cmp(&b.0));

        let mut header_bottom = None;

        let grid_response = egui::Grid::new("robot_path_control_grid")
            .striped(true)
            .spacing([12.0, 6.0])
            .show(ui, |ui| {
                let header_resp = ui.label(egui::RichText::new("Robot").strong());
                header_bottom = Some(header_resp.rect.bottom());
                ui.label(egui::RichText::new("Position").strong());
                ui.label(egui::RichText::new("Status").strong());
                ui.label(egui::RichText::new("Goal").strong());
                ui.label("");
                ui.end_row();

                for (robot_name, robot_pos) in &robot_list {
                    ui.label(robot_name);

                    ui.label(format!("({:.2}, {:.2})", robot_pos.x, robot_pos.y));

                    let path_data = self.path_state.0.get(robot_name);

                    let selected_dest = self.control_state.selected_destinations.get(robot_name);
                    let current_dest = self.control_state.current_destinations.get(robot_name);

                    // Update Status column
                    let (status_text, status_color) = match path_data {
                        // Robot is on path that is not completed
                        Some(path_data) if !path_data.is_completed() => ("Moving", MOVING_COLOR),

                        // Robot has completed path and there is no next destination queued
                        Some(_) if current_dest.is_some() && selected_dest.is_none() => {
                            ("Reached", REACHED_COLOR)
                        }

                        // Robot has no path data
                        _ => ("Pending", PENDING_COLOR),
                    };
                    ui.label(egui::RichText::new(status_text).color(status_color));

                    // Update Goal column
                    let (goal_text, goal_color) = match (selected_dest, current_dest) {
                        // Robot has pending destination
                        (Some(dest), _) => (dest.display_label(), PENDING_COLOR),

                        // Robot has no selected path but has completed destination
                        (None, Some(dest)) => (dest.display_label(), status_color),

                        // Robot has no destination
                        (None, None) => ("None".to_string(), Color32::WHITE),
                    };
                    ui.label(egui::RichText::new(goal_text).color(goal_color));

                    // Highlight robot that is currently being routed
                    let is_routing = self.control_state.active_routing_robot.as_deref()
                        == Some(robot_name.as_str());
                    if ui
                        .add(egui::Button::new("Route").selected(is_routing))
                        .clicked()
                    {
                        if is_routing {
                            self.control_state.active_routing_robot = None;
                        } else {
                            self.control_state.active_routing_robot = Some(robot_name.clone());
                        }
                    }
                    ui.end_row();
                }
            });

        if let Some(y) = header_bottom {
            ui.painter().hline(
                grid_response.response.rect.x_range(),
                y + 3.0,
                egui::Stroke::new(1.0_f32, Color32::WHITE),
            );
        }

        ui.add_space(8.0);
        ui.allocate_ui_with_layout(
            egui::vec2(grid_response.response.rect.width(), 0.0),
            egui::Layout::right_to_left(egui::Align::TOP),
            |ui| {
                ui.add_enabled(
                    !self.control_state.selected_destinations.is_empty(),
                    egui::Button::new("Send Scenario"),
                );
            },
        );
    }
}
