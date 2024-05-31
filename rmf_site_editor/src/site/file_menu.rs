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

use crate::menu_bar::{FileMenu, MenuEvent, MenuItem, MenuVisualizationStates};
use crate::{AppState, ExportFormat, SaveWorkspace, SaveWorkspaceDestination};
use bevy::prelude::*;
use std::collections::HashSet;

/// Keeps track of which entity is associated to the export sdf button.
#[derive(Resource)]
pub struct ExportSdfMenu {
    export_sdf: Entity,
}

impl ExportSdfMenu {
    pub fn get(&self) -> Entity {
        self.export_sdf
    }
}

impl FromWorld for ExportSdfMenu {
    fn from_world(world: &mut World) -> Self {
        let site_states = HashSet::from([
            AppState::SiteEditor,
            AppState::SiteVisualizer,
            AppState::SiteDrawingEditor,
        ]);
        let file_header = world.resource::<FileMenu>().get();
        let export_sdf = world
            .spawn((
                MenuItem::Text("Export Sdf".to_string()),
                MenuVisualizationStates(site_states),
            ))
            .set_parent(file_header)
            .id();

        ExportSdfMenu { export_sdf }
    }
}

pub fn handle_export_sdf_menu_events(
    mut menu_events: EventReader<MenuEvent>,
    sdf_menu: Res<ExportSdfMenu>,
    mut save_events: EventWriter<SaveWorkspace>,
) {
    for event in menu_events.read() {
        if event.clicked() && event.source() == sdf_menu.get() {
            save_events.send(SaveWorkspace {
                destination: SaveWorkspaceDestination::Dialog,
                format: ExportFormat::Sdf,
            });
        }
    }
}

#[derive(Default)]
pub struct SiteFileMenuPlugin;

impl Plugin for SiteFileMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ExportSdfMenu>().add_systems(
            Update,
            handle_export_sdf_menu_events.run_if(AppState::in_site_mode()),
        );
    }
}
