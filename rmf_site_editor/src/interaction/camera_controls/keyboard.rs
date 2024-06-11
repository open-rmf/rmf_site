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

use super::{CameraCommandType, CameraControls, CursorCommand, ProjectionMode};
use bevy::{prelude::*, window::PrimaryWindow};

#[derive(Resource)]
pub struct KeyboardCommand {
    pub translation_delta: Vec3,
    pub rotation_delta: Quat,
    pub scale_delta: f32,
    pub fov_delta: f32,
    pub keyboard_motion: Vec2,
    pub zoom_motion: f32,
    pub command_type: CameraCommandType,
}

impl Default for KeyboardCommand {
    fn default() -> Self {
        Self {
            translation_delta: Vec3::ZERO,
            rotation_delta: Quat::IDENTITY,
            scale_delta: 0.0,
            fov_delta: 0.0,
            keyboard_motion: Vec2::ZERO,
            zoom_motion: 0.0,
            command_type: CameraCommandType::Inactive,
        }
    }
}

pub fn update_keyboard_command(
    camera_controls: Res<CameraControls>,
    cursor_command: ResMut<CursorCommand>,
    mut keyboard_command: ResMut<KeyboardCommand>,
    keyboard_input: Res<Input<KeyCode>>,
    cameras: Query<(&Projection, &Transform)>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
) {
    if let Ok(window) = primary_windows.get_single() {
        // User inputs
        let is_shifting = keyboard_input.pressed(KeyCode::ShiftLeft)
            || keyboard_input.pressed(KeyCode::ShiftRight);
        let mut target_keyboard_motion = Vec2::ZERO;
        if keyboard_input.pressed(KeyCode::W) {
            target_keyboard_motion.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::A) {
            target_keyboard_motion.x += -1.0;
        }
        if keyboard_input.pressed(KeyCode::S) {
            target_keyboard_motion.y += -1.0;
        }
        if keyboard_input.pressed(KeyCode::D) {
            target_keyboard_motion.x += 1.0;
        }
        if target_keyboard_motion.length() > 0.0 {
            target_keyboard_motion = target_keyboard_motion.normalize();
        }

        let mut target_zoom_motion = 0.0;
        if keyboard_input.pressed(KeyCode::Q) {
            target_zoom_motion += -1.0;
        }
        if keyboard_input.pressed(KeyCode::E) {
            target_zoom_motion += 1.0;
        }

        // Smoothen motion using current state
        // (1 / reponse_factor) frames = number of frames to reach maximum velocity
        let response_factor = 0.05;
        let prev_keyboard_motion = keyboard_command.keyboard_motion;
        let mut keyboard_motion = prev_keyboard_motion
            + (target_keyboard_motion - prev_keyboard_motion).normalize_or_zero() * response_factor;
        if keyboard_motion.length() > 1.0 {
            keyboard_motion = keyboard_motion.normalize();
        } else if keyboard_motion.length() < 0.1 {
            keyboard_motion = Vec2::ZERO;
        }

        let prev_zoom_motion = keyboard_command.zoom_motion;
        let mut zoom_motion =
            prev_zoom_motion + (target_zoom_motion - prev_zoom_motion).signum() * response_factor;
        if zoom_motion.abs() > 1.0 {
            zoom_motion = zoom_motion.signum();
        } else if zoom_motion.abs() < 0.1 {
            zoom_motion = 0.0;
        }

        // Get command type
        let command_type = if is_shifting && keyboard_motion.length() > 0.0 {
            CameraCommandType::Orbit
        } else if keyboard_motion.length() > 0.0 {
            CameraCommandType::Pan
        } else if is_shifting && zoom_motion != 0.0 {
            CameraCommandType::FovZoom
        } else if zoom_motion != 0.0 {
            CameraCommandType::TranslationZoom
        } else {
            CameraCommandType::Inactive
        };

        if command_type != keyboard_command.command_type
            && command_type != CameraCommandType::Inactive
        {}
        {
            zoom_motion = response_factor * target_zoom_motion;
            keyboard_motion = response_factor * target_keyboard_motion;
        }

        // Camera projection and transform
        let active_camera_entity = match camera_controls.mode() {
            ProjectionMode::Orthographic => camera_controls.orthographic_camera_entities[0],
            ProjectionMode::Perspective => camera_controls.perspective_camera_entities[0],
        };
        let (camera_proj, camera_transform) = cameras.get(active_camera_entity).unwrap();

        // Orthographic
        match camera_controls.mode() {
            ProjectionMode::Orthographic => {
                *keyboard_command = update_orthographic_command(
                    command_type,
                    &camera_proj,
                    &camera_transform,
                    keyboard_motion,
                    zoom_motion,
                )
            }
            ProjectionMode::Perspective => {
                *keyboard_command = update_perspective_command(
                    command_type,
                    &camera_proj,
                    &camera_transform,
                    keyboard_motion,
                    zoom_motion,
                    window,
                )
            }
        }
    }
}

fn update_orthographic_command(
    command_type: CameraCommandType,
    camera_proj: &Projection,
    camera_transform: &Transform,
    keyboard_motion: Vec2,
    zoom_motion: f32,
) -> KeyboardCommand {
    let zoom_sensitivity = 0.05;
    let orbit_sensitivity = 2.0;
    let pan_sensitivity = 2.0;
    let mut keyboard_command = KeyboardCommand::default();

    // Zoom by scaling
    let mut target_scale = 0.0;
    if let Projection::Orthographic(camera_proj) = camera_proj {
        keyboard_command.scale_delta = zoom_motion * camera_proj.scale * zoom_sensitivity;
        target_scale = camera_proj.scale + keyboard_command.scale_delta;
    }

    // Keyboard motion to scale
    let keyboard_motion_adj = keyboard_motion * pan_sensitivity;

    match command_type {
        CameraCommandType::Orbit => {
            let yaw = keyboard_motion.x * orbit_sensitivity;
            let yaw = Quat::from_rotation_z(-yaw);
            keyboard_command.rotation_delta = yaw;
        }
        CameraCommandType::Pan => {
            let right_translation = camera_transform.rotation * Vec3::X;
            let up_translation = camera_transform.rotation * Vec3::Y;

            keyboard_command.translation_delta =
                up_translation * keyboard_motion_adj.y + right_translation * keyboard_motion_adj.x;
        }
        _ => (),
    }

    keyboard_command.command_type = command_type;
    keyboard_command.keyboard_motion = keyboard_motion;
    keyboard_command.zoom_motion = zoom_motion;

    return keyboard_command;
}

fn update_perspective_command(
    command_type: CameraCommandType,
    camera_proj: &Projection,
    camera_transform: &Transform,
    keyboard_motion: Vec2,
    zoom_motion: f32,
    window: &Window,
) -> KeyboardCommand {
    let fov_zoom_sensitivity = 0.1;
    let orbit_sensitivity = 20.0;
    let pan_sensitivity = 1.0;
    let translation_zoom_sensitivity = pan_sensitivity;
    let mut keyboard_command = KeyboardCommand::default();

    let zoom_translation = camera_transform.forward() * zoom_motion * translation_zoom_sensitivity;

    match command_type {
        CameraCommandType::FovZoom => {
            if let Projection::Perspective(camera_proj) = camera_proj {
                let target_fov = (camera_proj.fov + zoom_motion * fov_zoom_sensitivity).clamp(
                    std::f32::consts::PI * 10.0 / 180.0,
                    std::f32::consts::PI * 170.0 / 180.0,
                );
                keyboard_command.fov_delta = target_fov - camera_proj.fov;
            }
        }
        CameraCommandType::TranslationZoom => {
            keyboard_command.translation_delta = zoom_translation;
        }
        CameraCommandType::Pan => {
            let keyboard_motion_adj = keyboard_motion * pan_sensitivity;
            let right_translation = camera_transform.rotation * Vec3::X;
            let up_translation = -camera_transform.rotation * Vec3::Y;
            keyboard_command.translation_delta =
                up_translation * keyboard_motion_adj.y + right_translation * keyboard_motion_adj.x;
        }
        CameraCommandType::Orbit => {
            let keyboard_motion_adj = keyboard_motion * orbit_sensitivity;
            let window_size = Vec2::new(window.width() as f32, window.height() as f32);
            let delta_x = keyboard_motion_adj.x / window_size.x * std::f32::consts::PI * 2.0;
            let delta_y = keyboard_motion_adj.y / window_size.y * std::f32::consts::PI;
            let yaw = Quat::from_rotation_z(-delta_x);
            let pitch = Quat::from_rotation_x(delta_y);

            let mut target_rotation = (yaw * camera_transform.rotation) * pitch;
            target_rotation = if Transform::from_rotation(target_rotation).up().dot(Vec3::Z) > 0.0 {
                target_rotation
            } else {
                yaw * camera_transform.rotation
            };

            let start_rotation = Mat3::from_quat(camera_transform.rotation);
            let target_rotation = Mat3::from_quat(target_rotation);
            keyboard_command.rotation_delta =
                Quat::from_mat3(&(start_rotation.inverse() * target_rotation));
            keyboard_command.translation_delta = zoom_translation;
        }
        _ => (),
    }

    keyboard_command.command_type = command_type;
    keyboard_command.keyboard_motion = keyboard_motion;
    keyboard_command.zoom_motion = zoom_motion;

    return keyboard_command;
}
