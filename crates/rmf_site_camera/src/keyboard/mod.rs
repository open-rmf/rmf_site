/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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
    ActiveCameraQuery, UserCameraDisplay, active_camera_maybe,
    resources::{CameraConfig, CameraControls},
};

use super::{CameraCommandType, MAX_FOV, MAX_SCALE, MIN_FOV, MIN_SCALE, ProjectionMode, utils::*};
use bevy_ecs::prelude::*;
use bevy_input::prelude::*;
use bevy_math::prelude::*;
use bevy_picking::prelude::*;
use bevy_reflect::Reflect;
use bevy_render::prelude::*;
use bevy_time::Time;
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_window::{PrimaryWindow, Window};
use tracing::warn;

// Keyboard control limits
pub const MIN_RESPONSE_TIME: f32 = 0.25; // [s] time taken to reach minimum input, or to reset
pub const MAX_RESPONSE_TIME: f32 = 1.5; // [s] time taken to reach maximum input
pub const MAX_FOV_DELTA: f32 = 0.5; // [rad/s]
pub const MAX_ANGULAR_VEL: f32 = 3.5; // [rad/s]
pub const MAX_TRANSLATION_VEL: f32 = 8.0; // [m/s]
pub const SCALE_ZOOM_SENSITIVITY: f32 = 0.035;
pub const ORTHOGRAPHIC_PAN_SENSITIVITY: f32 = 0.015;

#[derive(Resource, Reflect)]
#[reflect(Resource)]
pub struct KeyboardCommand {
    pub translation_delta: Vec3,
    pub rotation_delta: Quat,
    pub scale_delta: f32,
    pub fov_delta: f32,
    pub keyboard_motion: Vec2,
    pub zoom_motion: f32,
    pub camera_selection: Option<Vec3>,
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
            camera_selection: None,
            command_type: CameraCommandType::Inactive,
        }
    }
}

impl KeyboardCommand {
    pub fn take_translation_delta(&mut self) -> Vec3 {
        std::mem::replace(&mut self.translation_delta, Vec3::ZERO)
    }

    pub fn take_rotation_delta(&mut self) -> Quat {
        std::mem::replace(&mut self.rotation_delta, Quat::IDENTITY)
    }

    pub fn take_scale_delta(&mut self) -> f32 {
        std::mem::replace(&mut self.scale_delta, 0.0)
    }

    pub fn take_fov_delta(&mut self) -> f32 {
        std::mem::replace(&mut self.fov_delta, 0.0)
    }
}

pub(crate) fn update_keyboard_command(
    mut camera_config: ResMut<CameraConfig>,
    camera_controls: ResMut<CameraControls>,
    mut keyboard_command: ResMut<KeyboardCommand>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    cameras: Query<(&Camera, &Projection, &Transform, &GlobalTransform)>,
    active_camera: ActiveCameraQuery,
    mesh_ray_cast: MeshRayCast,
    time: Res<Time>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
    uncovered_window_area: Option<Res<UserCameraDisplay>>,
) {
    if let Ok(_) = primary_windows.single() {
        // User inputs
        let is_shifting = keyboard_input.pressed(KeyCode::ShiftLeft)
            || keyboard_input.pressed(KeyCode::ShiftRight);
        let mut target_keyboard_motion = Vec2::ZERO;
        if keyboard_input.pressed(camera_controls.up) {
            target_keyboard_motion.y += 1.0;
        }
        if keyboard_input.pressed(camera_controls.left) {
            target_keyboard_motion.x += -1.0;
        }
        if keyboard_input.pressed(camera_controls.down) {
            target_keyboard_motion.y += -1.0;
        }
        if keyboard_input.pressed(camera_controls.right) {
            target_keyboard_motion.x += 1.0;
        }
        if target_keyboard_motion.length() > 0.0 {
            target_keyboard_motion = target_keyboard_motion.normalize();
        }

        let mut target_zoom_motion = 0.0;
        if keyboard_input.pressed(camera_controls.zoom_out) {
            target_zoom_motion += -1.0;
        }
        if keyboard_input.pressed(camera_controls.zoom_in) {
            target_zoom_motion += 1.0;
        }

        // Smooth and normalize keyboard
        let delta_seconds = time.delta_secs();
        let prev_keyboard_motion = keyboard_command.keyboard_motion;
        let keyboard_motion_delta =
            (target_keyboard_motion - prev_keyboard_motion).normalize_or_zero();
        let keyboard_response_factor = if keyboard_motion_delta.dot(prev_keyboard_motion) > 0.0 {
            delta_seconds / MAX_RESPONSE_TIME
        } else {
            delta_seconds / MIN_RESPONSE_TIME
        };
        let mut keyboard_motion =
            prev_keyboard_motion + keyboard_motion_delta * keyboard_response_factor;
        if keyboard_motion.length() > 1.0 {
            keyboard_motion = keyboard_motion.normalize();
        } else if keyboard_motion.length() < keyboard_response_factor {
            keyboard_motion = Vec2::ZERO;
        }

        // Smooth and normalize zoom motion
        let prev_zoom_motion = keyboard_command.zoom_motion;
        let zoom_motion_delta = target_zoom_motion - prev_zoom_motion;
        let zoom_response_factor = if prev_zoom_motion.signum() == zoom_motion_delta.signum() {
            delta_seconds / MAX_RESPONSE_TIME
        } else {
            delta_seconds / MIN_RESPONSE_TIME
        };
        let mut zoom_motion = if zoom_motion_delta.abs() < zoom_response_factor {
            prev_zoom_motion
        } else {
            prev_zoom_motion + zoom_motion_delta.signum() * zoom_response_factor
        };
        zoom_motion = zoom_motion.clamp(-1.0, 1.0);
        if zoom_motion.abs() < zoom_response_factor {
            zoom_motion = 0.0;
        }

        // Get command type
        let is_keyboard_motion_active = keyboard_motion
            .length()
            .max(target_keyboard_motion.length())
            > 0.0;
        let command_type = if is_shifting && is_keyboard_motion_active {
            CameraCommandType::Orbit
        } else if is_keyboard_motion_active {
            CameraCommandType::Pan
        } else if is_shifting && zoom_motion != 0.0 {
            CameraCommandType::FovZoom
        } else if zoom_motion != 0.0 {
            CameraCommandType::TranslationZoom
        } else {
            CameraCommandType::Inactive
        };

        // Ignore previous motion if new command
        if command_type != keyboard_command.command_type
            && command_type != CameraCommandType::Inactive
        {
            zoom_motion = delta_seconds / MAX_RESPONSE_TIME * target_zoom_motion;
            keyboard_motion = delta_seconds / MAX_RESPONSE_TIME * target_keyboard_motion;
        }

        // Camera projection and transform
        let Ok(active_camera_e) = active_camera_maybe(&active_camera) else {
            return;
        };

        let (camera, camera_proj, camera_transform, camera_global_transform) =
            cameras.get(active_camera_e).unwrap();

        // Set camera selection as orbit center, discard once orbit operation complete
        let camera_selection = match keyboard_command.camera_selection {
            Some(camera_selection) => Ok(camera_selection),
            None => get_camera_selected_point(
                &camera,
                &camera_global_transform,
                uncovered_window_area,
                mesh_ray_cast,
            ),
        };

        let Ok(camera_selection) = camera_selection else {
            warn!(
                "Point could not be calculated for camera due to: {:#?}",
                camera_selection
            );
            return;
        };

        if command_type == CameraCommandType::Orbit {
            camera_config.orbit_center = Some(camera_selection);
        }
        if keyboard_command.command_type == CameraCommandType::Orbit
            && keyboard_command.command_type != command_type
        {
            camera_config.orbit_center = None;
        }

        match *active_camera.proj_mode {
            ProjectionMode::Orthographic => {
                if let Projection::Orthographic(camera_proj) = camera_proj {
                    *keyboard_command = get_orthographic_command(
                        command_type,
                        camera_proj,
                        camera_transform,
                        keyboard_motion,
                        zoom_motion,
                        delta_seconds,
                    )
                }
            }
            ProjectionMode::Perspective => {
                if let Projection::Perspective(camera_proj) = camera_proj {
                    *keyboard_command = get_perspective_command(
                        command_type,
                        camera_proj,
                        camera_transform,
                        camera_selection,
                        keyboard_motion,
                        zoom_motion,
                        delta_seconds,
                    )
                }
            }
        }
    }
}

fn get_orthographic_command(
    command_type: CameraCommandType,
    camera_proj: &OrthographicProjection,
    camera_transform: &Transform,
    keyboard_motion: Vec2,
    zoom_motion: f32,
    delta_seconds: f32,
) -> KeyboardCommand {
    let mut keyboard_command = KeyboardCommand::default();

    // Zoom by scaling
    let target_scale = (camera_proj.scale
        - zoom_motion * camera_proj.scale * SCALE_ZOOM_SENSITIVITY)
        .clamp(MIN_SCALE, MAX_SCALE);
    keyboard_command.scale_delta = target_scale - camera_proj.scale;

    match command_type {
        CameraCommandType::Orbit => {
            let yaw = -keyboard_motion.x * MAX_ANGULAR_VEL * delta_seconds;
            let yaw = Quat::from_rotation_z(yaw);
            keyboard_command.rotation_delta = yaw;
        }
        CameraCommandType::Pan => {
            let right_translation = camera_transform.rotation * Vec3::X;
            let up_translation = camera_transform.rotation * Vec3::Y;
            keyboard_command.translation_delta =
                up_translation * keyboard_motion.y + right_translation * keyboard_motion.x;
            keyboard_command.translation_delta *= ORTHOGRAPHIC_PAN_SENSITIVITY * camera_proj.scale;
        }
        _ => (),
    }

    keyboard_command.command_type = command_type;
    keyboard_command.keyboard_motion = keyboard_motion;
    keyboard_command.zoom_motion = zoom_motion;

    return keyboard_command;
}

fn get_perspective_command(
    command_type: CameraCommandType,
    camera_proj: &PerspectiveProjection,
    camera_transform: &Transform,
    camera_selection: Vec3,
    keyboard_motion: Vec2,
    zoom_motion: f32,
    delta_seconds: f32,
) -> KeyboardCommand {
    let mut keyboard_command = KeyboardCommand::default();

    let zoom_translation = (camera_selection - camera_transform.translation).normalize()
        * zoom_motion
        * zoom_distance_factor(camera_transform.translation, camera_selection)
        * MAX_TRANSLATION_VEL
        * delta_seconds;

    match command_type {
        CameraCommandType::FovZoom => {
            let target_fov = (camera_proj.fov - zoom_motion * MAX_FOV_DELTA * delta_seconds)
                .clamp(MIN_FOV.to_radians(), MAX_FOV.to_radians());
            keyboard_command.fov_delta = target_fov - camera_proj.fov;
        }
        CameraCommandType::TranslationZoom => keyboard_command.translation_delta = zoom_translation,
        CameraCommandType::Pan => {
            let right_translation = camera_transform.rotation * Vec3::X;
            let up_translation = -camera_transform.rotation * Vec3::Y;
            let pan_translation = (right_translation * keyboard_motion.x
                + up_translation * keyboard_motion.y)
                * MAX_TRANSLATION_VEL
                * delta_seconds;
            keyboard_command.translation_delta = pan_translation + zoom_translation;
        }
        CameraCommandType::Orbit => {
            let orbit_center = camera_selection;
            let pitch = keyboard_motion.y * MAX_ANGULAR_VEL * delta_seconds;
            let yaw = keyboard_motion.x * MAX_ANGULAR_VEL * delta_seconds;
            let zoom = zoom_motion * MAX_TRANSLATION_VEL * delta_seconds;

            let camera_transform_next =
                orbit_camera_around_point(camera_transform, camera_selection, pitch, yaw, zoom);
            keyboard_command.translation_delta =
                camera_transform_next.translation - camera_transform.translation;
            keyboard_command.rotation_delta =
                camera_transform.rotation.inverse() * camera_transform_next.rotation;
            keyboard_command.camera_selection = Some(orbit_center);
        }
        _ => (),
    }

    keyboard_command.command_type = command_type;
    keyboard_command.keyboard_motion = keyboard_motion;
    keyboard_command.zoom_motion = zoom_motion;
    return keyboard_command;
}
