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

/// Keeps track of which entity is associated to the export urdf button.
#[derive(Resource)]
pub struct ExportUrdfMenu {
    export_urdf: Entity,
}

impl FromWorld for ExportUrdfMenu {
    fn from_world(world: &mut World) -> Self {
        let workcell_states = HashSet::from([AppState::WorkcellEditor]);
        // TODO(luca) add shortcut text for Ctrl-E
        let file_header = world.resource::<FileMenu>().get();
        let export_urdf = world
            .spawn((
                MenuItem::Text("Export Urdf".to_string()),
                MenuVisualizationStates(workcell_states),
            ))
            .set_parent(file_header)
            .id();

        ExportUrdfMenu { export_urdf }
    }
}

pub fn handle_export_urdf_menu_events(
    mut menu_events: EventReader<MenuEvent>,
    urdf_menu: Res<ExportUrdfMenu>,
    mut save_events: EventWriter<SaveWorkspace>,
) {
    for event in menu_events.iter() {
        if event.clicked() && event.source() == urdf_menu.export_urdf {
            save_events.send(SaveWorkspace {
                destination: SaveWorkspaceDestination::Dialog,
                format: ExportFormat::Urdf,
            });
        }
    }
}
