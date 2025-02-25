/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use crate::widgets::{HeaderTilePlugin, Tile, WidgetSystem};
use crate::{interaction::AnchorSelection, AppState};

use bevy::ecs::system::{SystemParam, SystemState};
use bevy::prelude::*;
use bevy_egui::egui::{Button, Ui};

/// Add a widget for lane creation
#[derive(Default)]
pub struct LaneCreationPlugin {}

impl Plugin for LaneCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<LaneCreation>::new());
    }
}

#[derive(SystemParam)]
struct LaneCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for LaneCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "↔", "Lane") {
                params.anchor_selection.create_lanes();
            }
        }
    }
}

/// Add widget for location creation
#[derive(Default)]
pub struct LocationCreationPlugin {}

impl Plugin for LocationCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<LocationCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct LocationCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for LocationCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "📌", "Location") {
                params.anchor_selection.create_location();
            }
        }
    }
}

/// Add widget for wall creation
#[derive(Default)]
pub struct WallCreationPlugin {}

impl Plugin for WallCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<WallCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct WallCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for WallCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "■", "Wall") {
                params.anchor_selection.create_walls();
            }
        }
    }
}

/// Add widget for door creation
#[derive(Default)]
pub struct DoorCreationPlugin {}

impl Plugin for DoorCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<DoorCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct DoorCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for DoorCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "🚪", "Door") {
                params.anchor_selection.create_door();
            }
        }
    }
}

/// Add widget for lift creation
#[derive(Default)]
pub struct LiftCreationPlugin {}

impl Plugin for LiftCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<LiftCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct LiftCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for LiftCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "⬍", "Lift") {
                params.anchor_selection.create_lift();
            }
        }
    }
}

/// Add widget for floor creation
#[derive(Default)]
pub struct FloorCreationPlugin {}

impl Plugin for FloorCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<FloorCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct FloorCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for FloorCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "✏", "Floor") {
                params.anchor_selection.create_floor();
            }
        }
    }
}

/// Add widget for fiducial creation
#[derive(Default)]
pub struct FiducialCreationPlugin {}

impl Plugin for FiducialCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<FiducialCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct FiducialCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for FiducialCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        match params.app_state.get() {
            AppState::SiteEditor => {
                if button_clicked(ui, "☉", "Fiducial") {
                    params.anchor_selection.create_site_fiducial();
                }
            }
            AppState::SiteDrawingEditor => {
                if button_clicked(ui, "☉", "Fiducial") {
                    params.anchor_selection.create_drawing_fiducial();
                }
            }
            _ => {
                return;
            }
        }
    }
}

/// Add widget for measurement creation
#[derive(Default)]
pub struct MeasurementCreationPlugin {}

impl Plugin for MeasurementCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<MeasurementCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct MeasurementCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for MeasurementCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteDrawingEditor = params.app_state.get() {
            if button_clicked(ui, "📏", "Measurement") {
                params.anchor_selection.create_measurements();
            }
        }
    }
}

/// Helper funtion to display the button name on hover
fn button_clicked(ui: &mut Ui, icon: &str, tooltip: &str) -> bool {
    ui.add(Button::new(icon)).on_hover_text(tooltip).clicked()
}
