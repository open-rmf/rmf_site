/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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
    interaction::Selection,
    site::{AlignSiteDrawings, Delete},
    CreateNewWorkspace, CurrentWorkspace, WorkspaceLoader, WorkspaceSaver,
};
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_egui::EguiContexts;
use bevy_impulse::*;
use rmf_site_camera::resources::ProjectionMode;

#[derive(Debug, Clone, Copy, Resource)]
pub struct DebugMode(pub bool);

impl FromWorld for DebugMode {
    fn from_world(_: &mut World) -> Self {
        DebugMode(false)
    }
}

pub struct KeyboardInputPlugin;

impl Plugin for KeyboardInputPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<DebugMode>()
            .add_systems(Last, handle_keyboard_input);

        let keyboard_just_pressed =
            app.spawn_continuous_service(Last, keyboard_just_pressed_stream);

        app.insert_resource(KeyboardServices {
            keyboard_just_pressed,
        });
    }
}

fn handle_keyboard_input(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    selection: Res<Selection>,
    mut egui_context: EguiContexts,
    mut delete: EventWriter<Delete>,
    mut new_workspace: EventWriter<CreateNewWorkspace>,
    mut projection_mode: ResMut<ProjectionMode>,
    mut debug_mode: ResMut<DebugMode>,
    mut align_site: EventWriter<AlignSiteDrawings>,
    current_workspace: Res<CurrentWorkspace>,
    primary_windows: Query<Entity, With<PrimaryWindow>>,
    mut workspace_loader: WorkspaceLoader,
    mut workspace_saver: WorkspaceSaver,
) {
    let Some(egui_context) = primary_windows
        .single()
        .ok()
        .and_then(|w| egui_context.try_ctx_for_entity_mut(w))
    else {
        return;
    };
    let ui_has_focus = egui_context.wants_pointer_input()
        || egui_context.wants_keyboard_input()
        || egui_context.is_pointer_over_area();

    if ui_has_focus {
        return;
    }

    if keyboard_input.just_pressed(KeyCode::F2) {
        *projection_mode = ProjectionMode::Orthographic;
    }

    if keyboard_input.just_pressed(KeyCode::F3) {
        *projection_mode = ProjectionMode::Perspective;
    }

    if keyboard_input.just_pressed(KeyCode::Delete)
        || keyboard_input.just_pressed(KeyCode::Backspace)
    {
        if let Some(selection) = selection.0 {
            delete.write(Delete::new(selection));
        } else {
            warn!("No selected entity to delete");
        }
    }

    if keyboard_input.just_pressed(KeyCode::KeyD) {
        debug_mode.0 = !debug_mode.0;
        info!("Toggling debug mode: {debug_mode:?}");
    }

    // Ctrl keybindings
    if keyboard_input.any_pressed([KeyCode::ControlLeft, KeyCode::ControlRight]) {
        if keyboard_input.just_pressed(KeyCode::KeyS) {
            if keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]) {
                workspace_saver.save_to_dialog();
            } else {
                workspace_saver.save_to_default_file();
            }
        }

        if keyboard_input.just_pressed(KeyCode::KeyT) {
            if let Some(site) = current_workspace.root {
                align_site.write(AlignSiteDrawings(site));
            }
        }

        if keyboard_input.just_pressed(KeyCode::KeyE) {
            workspace_saver.export_sdf_to_dialog();
        }

        // TODO(luca) pop up a confirmation prompt if the current file is not saved, or create a
        // gui to switch between open workspaces
        if keyboard_input.just_pressed(KeyCode::KeyN) {
            new_workspace.write(CreateNewWorkspace);
        }

        if keyboard_input.just_pressed(KeyCode::KeyO) {
            workspace_loader.load_from_dialog();
        }
    }
}

pub fn keyboard_just_pressed_stream(
    In(ContinuousService { key }): ContinuousServiceInput<(), (), StreamOf<KeyCode>>,
    mut orders: ContinuousQuery<(), (), StreamOf<KeyCode>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let Some(mut orders) = orders.get_mut(&key) else {
        return;
    };

    if orders.is_empty() {
        return;
    }

    for key_code in keyboard_input.get_just_pressed() {
        orders.for_each(|order| order.streams().send(StreamOf(*key_code)));
    }
}

#[derive(Resource)]
pub struct KeyboardServices {
    pub keyboard_just_pressed: Service<(), (), StreamOf<KeyCode>>,
}
