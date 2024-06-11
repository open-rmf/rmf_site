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

use super::{
    CameraCommandType, CameraControls, CursorCommand, ProjectionMode, MAX_PITCH, MAX_SELECTION_DIST,
};
use bevy::{math::Vec3A, prelude::*, render::camera, window::PrimaryWindow};
use bevy_mod_raycast::{
    immediate::{Raycast, RaycastSettings, RaycastVisibility},
    primitives::Ray3d,
};

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

pub fn update_keyboard_command(
    mut camera_controls: ResMut<CameraControls>,
    mut keyboard_command: ResMut<KeyboardCommand>,
    keyboard_input: Res<Input<KeyCode>>,
    cameras: Query<(&Projection, &Transform)>,
    mut immediate_raycast: Raycast,
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
        // (1 / reponse_factor) = number of frames to reach maximum keyboard input
        let response_factor = 0.1;
        let prev_keyboard_motion = keyboard_command.keyboard_motion;
        let mut keyboard_motion = prev_keyboard_motion
            + (target_keyboard_motion - prev_keyboard_motion).normalize_or_zero() * response_factor;
        if keyboard_motion.length() > 1.0 {
            keyboard_motion = keyboard_motion.normalize();
        } else if keyboard_motion.length() < response_factor {
            keyboard_motion = Vec2::ZERO;
        }

        let prev_zoom_motion = keyboard_command.zoom_motion;
        let zoom_motion_delta = if (target_zoom_motion - prev_zoom_motion).abs() < response_factor {
            0.0
        } else {
            (target_zoom_motion - prev_zoom_motion).signum()
        };
        let mut zoom_motion =
            prev_zoom_motion + zoom_motion_delta * response_factor;
        if zoom_motion.abs() > 1.0 {
            zoom_motion = zoom_motion.signum();
        } else if zoom_motion.abs() < response_factor {
            zoom_motion = 0.0;
        }

        // Get command type
        let is_keyboard_motion_active = keyboard_motion.length().max(target_keyboard_motion.length()) > 0.0;
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
        if command_type != keyboard_command.command_type && command_type != CameraCommandType::Inactive {
            zoom_motion = response_factor * target_zoom_motion;
            keyboard_motion = response_factor * target_keyboard_motion;
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
            None => get_camera_selected_point(camera_transform, &mut immediate_raycast),
        };
        if command_type == CameraCommandType::Orbit {
            camera_controls.orbit_center = Some(camera_selection);
        }
        if keyboard_command.command_type == CameraCommandType::Orbit && keyboard_command.command_type != command_type {
            camera_controls.orbit_center = None;
        }

        // Orthographic
        match camera_controls.mode() {
            ProjectionMode::Orthographic => {
                *keyboard_command = get_orthographic_command(
                    command_type,
                    camera_proj,
                    camera_transform,
                    keyboard_motion,
                    zoom_motion,
                )
            }
            ProjectionMode::Perspective => {
                *keyboard_command = get_perspective_command(
                    command_type,
                    camera_proj,
                    camera_transform,
                    camera_selection,
                    keyboard_motion,
                    zoom_motion,
                )
            }
        }

    }
}

fn get_orthographic_command(
    command_type: CameraCommandType,
    camera_proj: &Projection,
    camera_transform: &Transform,
    keyboard_motion: Vec2,
    zoom_motion: f32,
) -> KeyboardCommand {
    let pan_sensitivity = 0.015;
    let orbit_sensitivity = 0.04;
    let scale_zoom_sensitivity = 0.035;

    let mut keyboard_command = KeyboardCommand::default();

    if let Projection::Orthographic(camera_proj) = camera_proj {
        // Zoom by scaling
        keyboard_command.scale_delta = -zoom_motion * camera_proj.scale * scale_zoom_sensitivity;

        match command_type {
            CameraCommandType::Orbit => {
                let yaw = -keyboard_motion.x * orbit_sensitivity;
                let yaw = Quat::from_rotation_z(yaw);
                keyboard_command.rotation_delta = yaw;
            }
            CameraCommandType::Pan => {
                let right_translation = camera_transform.rotation * Vec3::X;
                let up_translation = camera_transform.rotation * Vec3::Y;

                keyboard_command.translation_delta =
                    up_translation * keyboard_motion.y + right_translation * keyboard_motion.x;
                keyboard_command.translation_delta *= pan_sensitivity * camera_proj.scale;
            }
            _ => (),
        }

        keyboard_command.command_type = command_type;
        keyboard_command.keyboard_motion = keyboard_motion;
        keyboard_command.zoom_motion = zoom_motion;
    }
    return keyboard_command;
}

fn get_perspective_command(
    command_type: CameraCommandType,
    camera_proj: &Projection,
    camera_transform: &Transform,
    camera_selection: Vec3,
    keyboard_motion: Vec2,
    zoom_motion: f32,
) -> KeyboardCommand {
    let pan_sensitivity = 0.1;
    let orbit_sensitivity = 0.005;
    let fov_zoom_sensitivity = 0.03;
    let translation_zoom_sensitivity = 0.1;

    let mut keyboard_command = KeyboardCommand::default();

    if let Projection::Perspective(camera_proj) = camera_proj {
        // Scale zoom by distance to object in camera center
        let dist_to_selection = (camera_transform.translation - camera_selection)
            .length()
            .max(1.0);
        let zoom_translation = camera_transform.forward()
            * (zoom_motion * translation_zoom_sensitivity)
            * (dist_to_selection * 0.2);

        match command_type {
            CameraCommandType::FovZoom => {
                let target_fov = (camera_proj.fov - zoom_motion * fov_zoom_sensitivity).clamp(
                    std::f32::consts::PI * 10.0 / 180.0,
                    std::f32::consts::PI * 170.0 / 180.0,
                );
                keyboard_command.fov_delta = target_fov - camera_proj.fov;
            }
            CameraCommandType::TranslationZoom => {
                keyboard_command.translation_delta = zoom_translation
            }
            CameraCommandType::Pan => {
                let keyboard_motion_adj = keyboard_motion * pan_sensitivity * camera_proj.fov;
                let right_translation = camera_transform.rotation * Vec3::X;
                let up_translation = -camera_transform.rotation * Vec3::Y;
                keyboard_command.translation_delta = up_translation * keyboard_motion_adj.y
                    + right_translation * keyboard_motion_adj.x + zoom_translation;
            }
            CameraCommandType::Orbit => {
                let mut target_transform = camera_transform.clone();
                let keyboard_motion_adj = keyboard_motion * orbit_sensitivity * camera_proj.fov;
                let delta_x = keyboard_motion_adj.x * std::f32::consts::PI * 2.0;
                let delta_y = keyboard_motion_adj.y * std::f32::consts::PI;
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
                let orbit_radius = (orbit_center - (camera_transform.translation + zoom_translation)).length();
                target_transform.translation = orbit_center + target_rotation * Vec3::new(0.0, 0.0, orbit_radius);


                let start_rotation = Mat3::from_quat(camera_transform.rotation);
                keyboard_command.rotation_delta =
                    Quat::from_mat3(&(start_rotation.inverse() * target_rotation));
                keyboard_command.translation_delta = (target_transform.translation - camera_transform.translation);
                keyboard_command.camera_selection = Some(orbit_center);
            }
            _ => (),
        }

        keyboard_command.command_type = command_type;
        keyboard_command.keyboard_motion = keyboard_motion;
        keyboard_command.zoom_motion = zoom_motion;
    }
    return keyboard_command;
}

pub fn get_camera_selected_point(
    camera_transform: &Transform,
    immediate_raycast: &mut Raycast,
) -> Vec3 {
    let camera_ray = Ray3d::new(camera_transform.translation, camera_transform.forward());
    let raycast_setting = RaycastSettings::default()
        .always_early_exit()
        .with_visibility(RaycastVisibility::MustBeVisible);

    let intersections = immediate_raycast.cast_ray(camera_ray, &raycast_setting);

    //TODO(@reuben-thomas) Filter for selectable entities
    if (intersections.len() > 0) {
        let (_, intersection_data) = &intersections[0];
        return intersection_data.position();
    }

    // If valid intersection with groundplane
    let denom = Vec3::Z.dot(camera_transform.forward());
    if denom.abs() > f32::EPSILON {
        let dist = (-1.0 * camera_transform.translation).dot(Vec3::Z) / denom;
        if dist > f32::EPSILON && dist < MAX_SELECTION_DIST {
            return camera_transform.translation + camera_transform.forward() * dist;
        }
    }

    // No groundplne intersection
    let height = camera_transform.translation.y.abs();
    let radius = if height < 1.0 { 1.0 } else { height };
    return camera_transform.translation + camera_transform.forward() * radius;
}
