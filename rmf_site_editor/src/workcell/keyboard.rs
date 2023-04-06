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


use bevy::prelude::*;
use bevy_egui::EguiContext;

use rmf_site_format::{WorkcellVisualMarker, WorkcellCollisionMarker};

pub fn handle_workcell_keyboard_input(
    keyboard_input: Res<Input<KeyCode>>,
    mut egui_context: ResMut<EguiContext>,
    mut visuals: Query<&mut Visibility, (With<WorkcellVisualMarker>, Without<WorkcellCollisionMarker>)>,
    mut collisions: Query<&mut Visibility, (With<WorkcellCollisionMarker>, Without<WorkcellVisualMarker>)>,
) {
    let egui_context = egui_context.ctx_mut();
    let ui_has_focus = egui_context.wants_pointer_input()
        || egui_context.wants_keyboard_input()
        || egui_context.is_pointer_over_area();

    if ui_has_focus {
        return;
    }

    if keyboard_input.any_pressed([KeyCode::LShift, KeyCode::RShift]) {
        if keyboard_input.just_pressed(KeyCode::V) {
            println!("Toggling visuals");
            for mut v in visuals.iter_mut() {
                v.is_visible = !v.is_visible;
            }
        }

        if keyboard_input.just_pressed(KeyCode::C) {
            println!("Toggling collisions");
            for mut c in collisions.iter_mut() {
                c.is_visible = !c.is_visible;
            }
        }
    }

}

