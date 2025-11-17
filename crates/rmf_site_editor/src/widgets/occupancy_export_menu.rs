/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use crate::{AppState, WorkspaceSaver};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
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
                MenuItem::Text(TextMenuItem::new("Export Occupancy").shortcut("Ctrl-E")),
                ChildOf(file_header),
            ))
            .id();

        OccupancyExportMenu { export_sdf }
    }
}

fn handle_export_sdf_menu_events(
    mut menu_events: EventReader<MenuEvent>,
    occupancy_menu: Res<OccupancyExportMenu>,
    mut workspace_saver: WorkspaceSaver,
) {
    for event in menu_events.read() {
        if event.clicked() && event.source() == occupancy_menu.get() {
            workspace_saver.export_occupancy_to_dialog();
        }
    }
}

#[derive(Default)]
pub struct OccupancyExportMenuPlugin {}

impl Plugin for OccupancyExportMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<OccupancyExportMenu>().add_systems(
            Update,
            handle_export_sdf_menu_events.run_if(AppState::in_displaying_mode()),
        );
    }
}
