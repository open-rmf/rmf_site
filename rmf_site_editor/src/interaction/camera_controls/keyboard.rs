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

use super::{
    get_groundplane_else_default_selection, CameraCommandType, CameraControls, ProjectionMode,
    MAX_FOV, MAX_PITCH, MAX_SCALE, MIN_FOV, MIN_SCALE,
};
use bevy::{prelude::*, window::PrimaryWindow};
use bevy_mod_raycast::{
    immediate::{Raycast, RaycastSettings, RaycastVisibility},
    primitives::Ray3d,
};

// Keyboard control limits
pub const MIN_RESPONSE_TIME: f32 = 0.25; // [s] time taken to reach minimum input, or to reset
pub const MAX_RESPONSE_TIME: f32 = 1.5; // [s] time taken to reach maximum input
pub const MAX_FOV_DELTA: f32 = 0.5; // [rad/s]
pub const MAX_ANGULAR_VEL: f32 = 3.5; // [rad/s]
pub const MAX_TRANSLATION_VEL: f32 = 8.0; // [m/s]
pub const SCALE_ZOOM_SENSITIVITY: f32 = 0.035;
pub const ORTHOGRAPHIC_PAN_SENSITIVITY: f32 = 0.015;

#[derive(Resource)]
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

pub fn update_keyboard_command(
    mut camera_controls: ResMut<CameraControls>,
    mut keyboard_command: ResMut<KeyboardCommand>,
    keyboard_input: Res<Input<KeyCode>>,
    cameras: Query<(&Projection, &Transform)>,
    immediate_raycast: Raycast,
    time: Res<Time>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
) {
    if let Ok(_) = primary_windows.get_single() {
        // User inputs
        let is_shifting = keyboard_input.pressed(KeyCode::ShiftLeft)
            || keyboard_input.pressed(KeyCode::ShiftRight);
        let mut target_keyboard_motion = Vec2::ZERO;
        if keyboard_input.pressed(KeyCode::Up) {
            target_keyboard_motion.y += 1.0;
        }
        if keyboard_input.pressed(KeyCode::Left) {
            target_keyboard_motion.x += -1.0;
        }
        if keyboard_input.pressed(KeyCode::Down) {
            target_keyboard_motion.y += -1.0;
        }
        if keyboard_input.pressed(KeyCode::Right) {
            target_keyboard_motion.x += 1.0;
        }
        if target_keyboard_motion.length() > 0.0 {
            target_keyboard_motion = target_keyboard_motion.normalize();
        }

        let mut target_zoom_motion = 0.0;
        if keyboard_input.pressed(KeyCode::N) {
            target_zoom_motion += -1.0;
        }
        if keyboard_input.pressed(KeyCode::M) {
            target_zoom_motion += 1.0;
        }

        // Smooth and normalize keyboard
        let delta_seconds = time.delta_seconds();
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
        let active_camera_entity = match camera_controls.mode() {
            ProjectionMode::Orthographic => camera_controls.orthographic_camera_entities[0],
            ProjectionMode::Perspective => camera_controls.perspective_camera_entities[0],
        };
        let (camera_proj, camera_transform) = cameras.get(active_camera_entity).unwrap();

        // Set camera selection as orbit center, discard once orbit operation complete
        let camera_selection = match keyboard_command.camera_selection {
            Some(camera_selection) => camera_selection,
            None => get_camera_selected_point(camera_transform, immediate_raycast),
        };
        if command_type == CameraCommandType::Orbit {
            camera_controls.orbit_center = Some(camera_selection);
        }
        if keyboard_command.command_type == CameraCommandType::Orbit
            && keyboard_command.command_type != command_type
        {
            camera_controls.orbit_center = None;
        }

        match camera_controls.mode() {
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

    let zoom_distance_factor =
        (0.2 * (camera_transform.translation - camera_selection).length()).max(1.0);
    let zoom_translation = camera_transform.forward()
        * zoom_motion
        * zoom_distance_factor
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
            let mut target_transform = camera_transform.clone();
            let delta_x = keyboard_motion.x * MAX_ANGULAR_VEL * delta_seconds;
            let delta_y = keyboard_motion.y * MAX_ANGULAR_VEL * delta_seconds;
            let yaw = Quat::from_rotation_z(delta_x);
            let pitch = Quat::from_rotation_x(-delta_y);

            // Rotation
            // Exclude pitch if exceeds maximum angle
            target_transform.rotation = (yaw * camera_transform.rotation) * pitch;
            if target_transform.up().z.acos().to_degrees() > MAX_PITCH {
                target_transform.rotation = yaw * camera_transform.rotation;
            };

            // Translation around orbit center
            let target_rotation = Mat3::from_quat(target_transform.rotation);
            let orbit_center = camera_selection;
            let orbit_radius =
                (orbit_center - (camera_transform.translation + zoom_translation)).length();
            target_transform.translation =
                orbit_center + target_rotation * Vec3::new(0.0, 0.0, orbit_radius);

            let start_rotation = Mat3::from_quat(camera_transform.rotation);
            keyboard_command.rotation_delta =
                Quat::from_mat3(&(start_rotation.inverse() * target_rotation));
            keyboard_command.translation_delta =
                target_transform.translation - camera_transform.translation;
            keyboard_command.camera_selection = Some(orbit_center);
        }
        _ => (),
    }

    keyboard_command.command_type = command_type;
    keyboard_command.keyboard_motion = keyboard_motion;
    keyboard_command.zoom_motion = zoom_motion;
    return keyboard_command;
}

pub fn get_camera_selected_point(
    camera_transform: &Transform,
    mut immediate_raycast: Raycast,
) -> Vec3 {
    let camera_ray = Ray3d::new(camera_transform.translation, camera_transform.forward());
    let raycast_setting = RaycastSettings::default()
        .always_early_exit()
        .with_visibility(RaycastVisibility::MustBeVisible);

    //TODO(@reuben-thomas) Filter for selectable entities
    let intersections = immediate_raycast.cast_ray(camera_ray, &raycast_setting);
    if intersections.len() > 0 {
        let (_, intersection_data) = &intersections[0];
        return intersection_data.position();
    } else {
        return get_groundplane_else_default_selection(
            camera_ray.origin(),
            camera_ray.direction(),
            camera_ray.direction(),
        );
    }
}
