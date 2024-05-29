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

use super::{CameraCommandType, CameraControls, ProjectionMode};
use crate::interaction::SiteRaycastSet;
use bevy::ecs::component::TableStorage;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::render::camera;
use bevy::window::PrimaryWindow;
use bevy_mod_raycast::deferred::RaycastSource;
use nalgebra::{Matrix3, Matrix3x1};

#[derive(Resource)]
pub struct CursorCommand {
    pub translation_delta: Vec3,
    pub rotation_delta: Quat,
    pub scale_delta: f32,
    pub cursor_selection: Option<Vec3>,
    pub command_type: CameraCommandType,
}

impl Default for CursorCommand {
    fn default() -> Self {
        Self {
            translation_delta: Vec3::ZERO,
            rotation_delta: Quat::IDENTITY,
            scale_delta: 0.0,
            cursor_selection: None,
            command_type: CameraCommandType::Inactive,
        }
    }
}

pub fn update_cursor_command(
    camera_controls: Res<CameraControls>,
    mut cursor_command: ResMut<CursorCommand>,
    mut mouse_motion: EventReader<MouseMotion>,
    mut mouse_wheel: EventReader<MouseWheel>,
    mouse_input: Res<Input<MouseButton>>,
    keyboard_input: Res<Input<KeyCode>>,
    raycast_sources: Query<&RaycastSource<SiteRaycastSet>>,
    cameras: Query<(&Projection, &Transform, &GlobalTransform)>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
) {
    if let Ok(window) = primary_windows.get_single() {
        // Cursor and scroll inputs
        let mut cursor_motion = mouse_motion
            .read()
            .map(|event| event.delta)
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

        // Command type, return if inactive
        let command_type = get_command_type(
            &keyboard_input,
            &mouse_input,
            &cursor_motion,
            &scroll_motion,
        );
        if command_type == CameraCommandType::Inactive {
            *cursor_command = CursorCommand::default();
            return;
        }

        // Camera projection and transform
        let active_camera_entity = match camera_controls.mode() {
            ProjectionMode::Orthographic => camera_controls.orthographic_camera_entities[0],
            ProjectionMode::Perspective => camera_controls.perspective_camera_entities[0],
        };
        let (camera_proj, camera_transform, camera_global_transform) =
            cameras.get(active_camera_entity).unwrap();

        // Get selection under cursor, cursor direction
        let Ok(cursor_raycast_source) = raycast_sources.get_single() else {
            return;
        };
        let cursor_ray = match cursor_raycast_source.get_ray() {
            Some(ray) => ray,
            None => return,
        };
        let cursor_selection = match cursor_command.cursor_selection {
            Some(selection) => selection,
            None => get_cursor_selected_point(&cursor_raycast_source),
        };
        let cursor_direction = cursor_ray.direction().normalize();

        // 4. Perspective Mode
        *cursor_command = match camera_controls.mode() {
            ProjectionMode::Perspective => get_perspective_cursor_command(
                &camera_transform,
                &camera_proj,
                command_type,
                cursor_motion,
                cursor_direction,
                cursor_selection,
                scroll_motion,
                window,
            ),
            ProjectionMode::Orthographic => get_orthographic_cursor_command(),
        };
    } else {
        *cursor_command = CursorCommand::default();
    }
}

fn get_orthographic_cursor_command() -> CursorCommand {
    CursorCommand::default()
}

fn get_perspective_cursor_command(
    camera_transform: &Transform,
    camera_proj: &Projection,
    command_type: CameraCommandType,
    cursor_motion: Vec2,
    cursor_direction: Vec3,
    cursor_selection: Vec3,
    scroll_motion: f32,
    window: &Window,
) -> CursorCommand {
    // Zoom towards the cursor if zooming only, otherwize zoom to center
    let zoom_translation = match command_type {
        CameraCommandType::TranslationZoom => cursor_direction * 0.5 * scroll_motion,
        _ => camera_transform.forward() * scroll_motion,
    };

    let mut translation_delta = Vec3::ZERO;
    let mut rotation_delta = Quat::IDENTITY;
    let mut scale_delta = 0.0;
    let mut is_cursor_selecting = false;

    match command_type {
        CameraCommandType::TranslationZoom => {
            translation_delta = zoom_translation;
        }
        CameraCommandType::Pan => {
            // To keep the same point below the cursor, we solve
            // selection_to_camera + translation_delta = selection_to_camera_next
            // selection_to_camera_next = x3 * -cursor_direction
            let selection_to_camera = cursor_selection - camera_transform.translation;

            // translation_delta = x1 * right_ transltion + x2 * up_translation
            let right_translation = camera_transform.rotation * Vec3::X;
            let up_translation = camera_transform.rotation * Vec3::Y;

            let a = Matrix3::new(
                right_translation.x,
                up_translation.x,
                -cursor_direction.x,
                right_translation.y,
                up_translation.y,
                -cursor_direction.y,
                right_translation.z,
                up_translation.z,
                -cursor_direction.z,
            );
            let b = Matrix3x1::new(
                selection_to_camera.x,
                selection_to_camera.y,
                selection_to_camera.z,
            );
            let x = a.lu().solve(&b).unwrap();

            translation_delta = zoom_translation + x[0] * right_translation + x[1] * up_translation;
            rotation_delta = Quat::IDENTITY;
            is_cursor_selecting = true;
        }
        CameraCommandType::Orbit => {
            let window_size = Vec2::new(window.width() as f32, window.height() as f32);
            let delta_x = cursor_motion.x / window_size.x * std::f32::consts::PI;
            let delta_y = cursor_motion.y / window_size.y * std::f32::consts::PI;

            let yaw = Quat::from_rotation_z(delta_x);
            let pitch = Quat::from_rotation_x(delta_y);

            let target_rotation = (yaw * camera_transform.rotation) * pitch;

            // Do not allow rotations to upside down states
            if Transform::from_rotation(target_rotation).up().dot(Vec3::Z) > 0.0 {
                let start_rotation = Mat3::from_quat(camera_transform.rotation);
                let target_rotation = Mat3::from_quat(target_rotation);
                rotation_delta = Quat::from_mat3(&(start_rotation.inverse() * target_rotation));
            }
            translation_delta = zoom_translation;
        }
        _ => (),
    }

    let cursor_selection = if is_cursor_selecting {
        Some(cursor_selection)
    } else {
        None
    };

    return CursorCommand {
        translation_delta,
        rotation_delta,
        scale_delta: 0.0,
        cursor_selection,
        command_type,
    };
}

// Returns the object selected by the cursor, if none, defaults to ground plane or arbitrary point in front
fn get_cursor_selected_point(cursor_raycast_source: &RaycastSource<SiteRaycastSet>) -> Vec3 {
    let cursor_ray = cursor_raycast_source.get_ray().unwrap();
    let default_dist = 100.0;
    match cursor_raycast_source.get_nearest_intersection() {
        Some((_, intersection)) => intersection.position(),
        None => {
            let n_p = Vec3::Z;
            let n_r = cursor_ray.direction();
            let denom = n_p.dot(n_r);
            if denom > 1e-3 {
                cursor_ray.origin() + (default_dist * cursor_ray.direction())
            } else {
                let t = (Vec3::Z - cursor_ray.origin()).dot(n_p) / denom;
                cursor_ray.origin() + t * cursor_ray.direction()
            }
        }
    }
}

fn get_command_type(
    keyboard_input: &Res<Input<KeyCode>>,
    mouse_input: &Res<Input<MouseButton>>,
    cursor_motion: &Vec2,
    scroll_motion: &f32,
) -> CameraCommandType {
    // Inputs
    let is_cursor_moving = cursor_motion.length() > 0.;
    let is_scrolling = *scroll_motion != 0.;
    let is_shifting =
        keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight);

    // Panning
    if is_cursor_moving && !is_shifting && mouse_input.pressed(MouseButton::Right) {
        return CameraCommandType::Pan;
    }

    // Orbitting
    if is_cursor_moving && mouse_input.pressed(MouseButton::Middle)
        || (mouse_input.pressed(MouseButton::Right) && is_shifting)
    {
        return CameraCommandType::Orbit;
    }

    // Zoom
    if is_scrolling {
        return CameraCommandType::TranslationZoom;
    }

    return CameraCommandType::Inactive;
}
