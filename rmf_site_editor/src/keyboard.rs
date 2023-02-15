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
    interaction::{
        camera_controls::{CameraControls, HeadlightToggle},
        ChangeMode, InteractionMode, Selection,
    },
    site::{CurrentWorkspace, Delete, SaveSite},
    workcell::SaveWorkcell,
    AppState,
};
use bevy::prelude::*;
use bevy_egui::EguiContext;

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
            .add_system(handle_keyboard_input);
    }
}

fn handle_keyboard_input(
    keyboard_input: Res<Input<KeyCode>>,
    selection: Res<Selection>,
    current_mode: Res<InteractionMode>,
    mut camera_controls: ResMut<CameraControls>,
    mut cameras: Query<&mut Camera>,
    mut visibilities: Query<&mut Visibility>,
    mut egui_context: ResMut<EguiContext>,
    mut change_mode: EventWriter<ChangeMode>,
    mut delete: EventWriter<Delete>,
    mut save_site: EventWriter<SaveSite>,
    mut save_workcell: EventWriter<SaveWorkcell>,
    headlight_toggle: Res<HeadlightToggle>,
    workspace: Res<CurrentWorkspace>,
    mut debug_mode: ResMut<DebugMode>,
    app_state: Res<State<AppState>>,
) {
    let egui_context = egui_context.ctx_mut();
    let ui_has_focus = egui_context.wants_pointer_input()
        || egui_context.wants_keyboard_input()
        || egui_context.is_pointer_over_area();

    if ui_has_focus {
        return;
    }

    if keyboard_input.just_pressed(KeyCode::F2) {
        camera_controls.use_orthographic(true, &mut cameras, &mut visibilities, headlight_toggle.0);
    }

    if keyboard_input.just_pressed(KeyCode::F3) {
        camera_controls.use_perspective(true, &mut cameras, &mut visibilities, headlight_toggle.0);
    }

    if keyboard_input.just_pressed(KeyCode::Escape) {
        change_mode.send(ChangeMode::Backout);
    }

    if keyboard_input.just_pressed(KeyCode::Delete) || keyboard_input.just_pressed(KeyCode::Back) {
        if current_mode.is_inspecting() {
            if let Some(selection) = selection.0 {
                delete.send(Delete::new(selection));
            } else {
                println!("No selected entity to delete");
            }
        }
    }

    if keyboard_input.just_pressed(KeyCode::D) {
        debug_mode.0 = !debug_mode.0;
        println!("Toggling debug mode: {debug_mode:?}");
    }

    // TODO(luca) Consider having a single SaveWorkspace event and doing the pattern matching there
    // instead of duplicating events, could have a root "save.rs" and "load.rs"
    if keyboard_input.any_pressed([KeyCode::LControl, KeyCode::RControl])
        && keyboard_input.just_pressed(KeyCode::S)
    {
        if let Some(ws_root) = workspace.root {
            match app_state.current() {
                AppState::WorkcellEditor => {
                    println!("Saving Workcell");
                    save_workcell.send(SaveWorkcell {
                        root: ws_root,
                        to_file: None,
                    });
                }
                AppState::SiteEditor => {
                    println!("Saving Site");
                    save_site.send(SaveSite {
                        site: ws_root,
                        to_file: None,
                    });
                }
                AppState::MainMenu => { /* Noop */ }
            }
        } else {
            println!("Unable to save, no workspace loaded");
        }
    }
}
