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

use bevy::prelude::*;
use bevy::input::mouse::{self, MouseMotion, MouseScrollUnit, MouseWheel};
use crate::interaction::PickingBlockers;

#[derive(Resource)]
pub struct MouseCommand {
    pub pan: Vec2,
    pub orbit: Vec2,
    pub orbit_state_changed: bool,
    pub was_orbitting: bool,
    pub zoom_target: Vec3,
    pub zoom_delta: f32,
}

impl Default for MouseCommand {
    fn default() -> Self {
        Self {
            pan: Vec2::ZERO,
            orbit: Vec2::ZERO,
            orbit_state_changed: false,
            was_orbitting: false,
            zoom_target: Vec3::ZERO,
            zoom_delta: 0.0
        }
    }
}

pub fn update_mouse_command(
    mut mouse_command: ResMut<MouseCommand>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    mouse_input: Res<Input<MouseButton>>,
    keyboard_input: Res<Input<KeyCode>>,
) {
    // Keyboard, mouse inputs
    let is_shifting =
        keyboard_input.pressed(KeyCode::ShiftLeft) || 
        keyboard_input.pressed(KeyCode::ShiftRight);
    let cursor_motion = mouse_motion.read().map(|event| event.delta)
        .fold(Vec2::ZERO, |acc, delta| acc + delta);
    let mut scroll_motion = 0.0;
    for ev in mouse_wheel.read() {
        #[cfg(not(target_arch = "wasm32"))]
        {
            scroll_motion += ev.y;
        }
        #[cfg(target_arch = "wasm32")]
        {
            // scrolling in wasm is a different beast
            scroll_motion += 0.4 * ev.y / ev.y.abs();
        }
    }

    // Pan
    let is_panning = mouse_input.pressed(MouseButton::Right) && !is_shifting;
    if is_panning {
        mouse_command.pan = cursor_motion;
    } else {
        mouse_command.pan = Vec2::ZERO;
    }

    // Orbit
    let is_orbitting = mouse_input.pressed(MouseButton::Middle) ||
        (mouse_input.pressed(MouseButton::Right) && is_shifting);
    mouse_command.orbit_state_changed = mouse_command.was_orbitting != is_orbitting;
    mouse_command.was_orbitting = is_orbitting; 
    if is_orbitting && !is_panning {
        mouse_command.orbit = cursor_motion;
    } else {
        mouse_command.orbit = Vec2::ZERO;
    }

    // Zoom
    mouse_command.zoom_delta = scroll_motion;
    if !is_orbitting && !is_panning {
        mouse_command.zoom_target = Vec3::new(0.0, 0.0, scroll_motion);
    } else {
        mouse_command.zoom_target = Vec3::ZERO;
    }
}





