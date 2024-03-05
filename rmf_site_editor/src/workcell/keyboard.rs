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

use crate::{ExportFormat, SaveWorkspace, SaveWorkspaceDestination};
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::EguiContexts;

pub fn handle_workcell_keyboard_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut egui_context: EguiContexts,
    mut save_events: EventWriter<SaveWorkspace>,
    primary_windows: Query<Entity, With<PrimaryWindow>>,
) {
    let Some(egui_context) = primary_windows
        .get_single()
        .ok()
        .and_then(|w| egui_context.try_ctx_for_window_mut(w))
    else {
        return;
    };

    let ui_has_focus = egui_context.wants_pointer_input()
        || egui_context.wants_keyboard_input()
        || egui_context.is_pointer_over_area();

    if ui_has_focus {
        return;
    }

    if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        if keyboard_input.just_pressed(KeyCode::E) {
            save_events.send(SaveWorkspace {
                destination: SaveWorkspaceDestination::Dialog,
                format: ExportFormat::Urdf,
            });
        }
    }
}
