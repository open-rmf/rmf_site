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

use crate::{
    AppState, WorkspaceSaver, occupancy::{CalculateGrid, ExportGridRequest, OccupancyInfo}
};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use bevy_egui::{
    egui::{self, DragValue},
    EguiContexts,
};
use rmf_site_egui::*;

/// Keeps track of which entity is associated to the export sdf button.
#[derive(Resource)]
pub struct OccupancyExportMenu {
    export_sdf: Entity,
}

impl OccupancyExportMenu {
    pub fn get(&self) -> Entity {
        self.export_sdf
    }
}

impl FromWorld for OccupancyExportMenu {
    fn from_world(world: &mut World) -> Self {
        let file_header = world.resource::<FileMenu>().get();
        let export_sdf = world
            .spawn((
                MenuItem::Text(TextMenuItem::new("Export Occupancy")),
                ChildOf(file_header),
            ))
            .id();

        OccupancyExportMenu { export_sdf }
    }
}

#[derive(Default)]
struct ExportOccupancyConfig {
    visible: bool,
}

fn handle_export_occupancy_menu_events(
    mut menu_events: EventReader<MenuEvent>,
    mut export_event: EventWriter<ExportGridRequest>,
    occupancy_menu: Res<OccupancyExportMenu>,
    mut occupancy_info: ResMut<OccupancyInfo>,
    mut egui_context: EguiContexts,
    mut configuration: Local<ExportOccupancyConfig>,
) {
    for event in menu_events.read() {
        if event.clicked() && event.source() == occupancy_menu.get() {
            configuration.visible = true;
        }
    }

    if !configuration.visible {
        return;
    }

    egui::Window::new("Occupancy Export Options").show(egui_context.ctx_mut(), |ui| {
        ui.horizontal(|ui| {
            ui.label("Cell Size:");
            ui.add(DragValue::new(
                &mut occupancy_info.cell_size,
            ));
        });
        if ui.button("Export").clicked() {
            configuration.visible = false;
            export_event.write(ExportGridRequest);
        }
    });
}

#[derive(Default)]
pub struct OccupancyExportMenuPlugin {}

impl Plugin for OccupancyExportMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OccupancyExportMenu>().add_systems(
            Update,
            handle_export_occupancy_menu_events.run_if(AppState::in_displaying_mode()),
        );
    }
}
