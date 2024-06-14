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
    CameraCommandType, CameraControls, ProjectionMode, MAX_FOV, MAX_PITCH, MAX_SELECTION_DIST,
    MIN_FOV,
};
use crate::interaction::SiteRaycastSet;
use bevy::input::mouse::{MouseMotion, MouseWheel};
use bevy::prelude::*;
use bevy::window::PrimaryWindow;
use bevy_mod_raycast::deferred::RaycastSource;
use nalgebra::{Matrix3, Matrix3x1};

#[derive(Resource)]
pub struct CursorCommand {
    pub translation_delta: Vec3,
    pub rotation_delta: Quat,
    pub scale_delta: f32,
    pub fov_delta: f32,
    pub cursor_selection: Option<Vec3>,
    pub cursor_direction_camera_frame: Option<Vec3>,
    pub command_type: CameraCommandType,
}

impl Default for CursorCommand {
    fn default() -> Self {
        Self {
            translation_delta: Vec3::ZERO,
            rotation_delta: Quat::IDENTITY,
            scale_delta: 0.0,
            fov_delta: 0.0,
            cursor_selection: None,
            cursor_direction_camera_frame: None,
            command_type: CameraCommandType::Inactive,
        }
    }
}

pub fn update_cursor_command(
    mut camera_controls: ResMut<CameraControls>,
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
        // Return if cursor not within window
        if window.cursor_position().is_none() {
            *cursor_command = CursorCommand::default();
            return;
        }

        // Cursor and scroll inputs
        let cursor_motion = mouse_motion
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
        let command_type_prev = cursor_command.command_type;
        let command_type = get_command_type(
            &keyboard_input,
            &mouse_input,
            &cursor_motion,
            &scroll_motion,
            camera_controls.mode(),
            &command_type_prev,
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
        let (camera_proj, camera_transform, _) = cameras.get(active_camera_entity).unwrap();

        // Get selection under cursor, cursor direction
        let Ok(cursor_raycast_source) = raycast_sources.get_single() else {
            return;
        };
        let cursor_ray = match cursor_raycast_source.get_ray() {
            Some(ray) => ray,
            None => return,
        };
        let cursor_selection_new = get_cursor_selected_point(&cursor_raycast_source);
        let cursor_selection = match cursor_command.cursor_selection {
            Some(selection) => selection,
            None => cursor_selection_new,
        };
        let cursor_direction = cursor_ray.direction().normalize();
        let cursor_direction_camera_frame = camera_transform.rotation.inverse() * cursor_direction;
        let cursor_direction_camera_frame_prev = cursor_command
            .cursor_direction_camera_frame
            .unwrap_or(cursor_direction_camera_frame);

        // Update orbit center
        let was_orbittting = command_type_prev == CameraCommandType::Orbit
            || command_type_prev == CameraCommandType::HoldOrbitSelection;
        let is_orbitting = command_type == CameraCommandType::Orbit
            || command_type == CameraCommandType::HoldOrbitSelection;
        if !is_orbitting {
            camera_controls.orbit_center = None;
        } else if !was_orbittting && is_orbitting {
            camera_controls.orbit_center = Some(cursor_selection);
        }

        match camera_controls.mode() {
            ProjectionMode::Orthographic => {
                if let Projection::Orthographic(camera_proj) = camera_proj {
                    *cursor_command = get_orthographic_cursor_command(
                        &camera_transform,
                        &camera_proj,
                        command_type,
                        cursor_selection,
                        cursor_selection_new,
                        scroll_motion,
                    );
                }
            }
            ProjectionMode::Perspective => {
                if let Projection::Perspective(camera_proj) = camera_proj {
                    *cursor_command = get_perspective_cursor_command(
                        &camera_transform,
                        &camera_proj,
                        command_type,
                        cursor_direction,
                        cursor_direction_camera_frame,
                        cursor_direction_camera_frame_prev,
                        cursor_selection,
                        scroll_motion,
                        camera_controls.orbit_center,
                        window,
                    );
                }
            }
        }
    } else {
        *cursor_command = CursorCommand::default();
    }
}

fn get_orthographic_cursor_command(
    camera_transform: &Transform,
    camera_proj: &OrthographicProjection,
    command_type: CameraCommandType,
    cursor_selection: Vec3,
    cursor_selection_new: Vec3,
    scroll_motion: f32,
) -> CursorCommand {
    let mut cursor_command = CursorCommand::default();
    let mut is_cursor_selecting = false;

    // Zoom
    cursor_command.scale_delta = -scroll_motion * camera_proj.scale * 0.1;

    //TODO(@reuben-thomas) Find out why cursor ray cannot be used for direction
    let cursor_direction = (cursor_selection_new - camera_transform.translation).normalize();

    match command_type {
        CameraCommandType::Pan => {
            let selection_to_camera = cursor_selection - camera_transform.translation;
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

            cursor_command.translation_delta = x[0] * right_translation + x[1] * up_translation;
            is_cursor_selecting = true;
        }
        CameraCommandType::Orbit => {
            let cursor_direction_prev =
                (cursor_selection - camera_transform.translation).normalize();

            let heading_0 =
                (cursor_direction_prev - cursor_direction_prev.project_onto(Vec3::Z)).normalize();
            let heading_1 = (cursor_direction - cursor_direction.project_onto(Vec3::Z)).normalize();
            let is_clockwise = heading_0.cross(heading_1).dot(Vec3::Z) > 0.0;
            let yaw = heading_0.angle_between(heading_1) * if is_clockwise { -1.0 } else { 1.0 };
            let yaw = Quat::from_rotation_z(yaw);

            cursor_command.rotation_delta = yaw;
            is_cursor_selecting = true;
        }
        _ => (),
    }

    cursor_command.command_type = command_type;
    cursor_command.cursor_selection = if is_cursor_selecting {
        Some(cursor_selection)
    } else {
        None
    };

    return cursor_command;
}

fn get_perspective_cursor_command(
    camera_transform: &Transform,
    camera_proj: &PerspectiveProjection,
    command_type: CameraCommandType,
    cursor_direction: Vec3,
    cursor_direction_camera_frame: Vec3,
    cursor_direction_camera_frame_prev: Vec3,
    cursor_selection: Vec3,
    scroll_motion: f32,
    orbit_center: Option<Vec3>,
    window: &Window,
) -> CursorCommand {
    let translation_zoom_sensitivity = 0.5;
    let fov_zoom_sensitivity = 0.1;

    let mut cursor_command = CursorCommand::default();
    let mut is_cursor_selecting = false;

    match command_type {
        CameraCommandType::FovZoom => {
            let target_fov = (camera_proj.fov - scroll_motion * fov_zoom_sensitivity)
                .clamp(MIN_FOV.to_radians(), MAX_FOV.to_radians());
            cursor_command.fov_delta = target_fov - camera_proj.fov;
        }
        CameraCommandType::TranslationZoom => {
            cursor_command.translation_delta =
                cursor_direction * translation_zoom_sensitivity * scroll_motion;
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

            let zoom_translation =
                camera_transform.forward() * translation_zoom_sensitivity * scroll_motion;

            cursor_command.translation_delta =
                zoom_translation + x[0] * right_translation + x[1] * up_translation;
            cursor_command.rotation_delta = Quat::IDENTITY;
            is_cursor_selecting = true;
        }
        CameraCommandType::Orbit => {
            // Pitch and yaw inputs from cursor direction vector as spherical coordinates
            let pitch_input = cursor_direction_camera_frame_prev.y.acos()
                - cursor_direction_camera_frame.y.acos();
            let yaw_input = (cursor_direction_camera_frame_prev
                .z
                .atan2(cursor_direction_camera_frame_prev.x))
                - (cursor_direction_camera_frame
                    .z
                    .atan2(cursor_direction_camera_frame.x));

            // Scale inputs to viewport range
            let pitch_viewport_range = camera_proj.fov;
            let pitch = pitch_input / pitch_viewport_range * std::f32::consts::PI;
            let pitch = Quat::from_rotation_x(pitch);
            let yaw_viewport_range = camera_proj.fov * camera_proj.aspect_ratio;
            let yaw = yaw_input / yaw_viewport_range * 2.0 * std::f32::consts::PI;
            let yaw = Quat::from_rotation_z(yaw);

            let mut target_transform = camera_transform.clone();

            // Rotation
            // Exclude pitch if exceeds maximum angle
            target_transform.rotation = (yaw * camera_transform.rotation) * pitch;
            if target_transform.up().z.acos().to_degrees() > MAX_PITCH {
                target_transform.rotation = yaw * camera_transform.rotation;
            };

            // Translation
            if let Some(orbit_center) = orbit_center {
                let camera_to_orbit_center = orbit_center - camera_transform.translation;
                let x = camera_to_orbit_center.dot(camera_transform.local_x());
                let y = camera_to_orbit_center.dot(camera_transform.local_y());
                let z = camera_to_orbit_center.dot(camera_transform.local_z());
                let camera_to_orbit_center_next = target_transform.local_x() * x
                    + target_transform.local_y() * y
                    + target_transform.local_z() * z;

                let zoom_translation = camera_to_orbit_center_next.normalize()
                    * translation_zoom_sensitivity
                    * scroll_motion;
                target_transform.translation =
                    orbit_center - camera_to_orbit_center_next - zoom_translation;
            }

            cursor_command.translation_delta =
                target_transform.translation - camera_transform.translation;
            let start_rotation = Mat3::from_quat(camera_transform.rotation);
            let target_rotation = Mat3::from_quat(target_transform.rotation);
            cursor_command.rotation_delta =
                Quat::from_mat3(&(start_rotation.inverse() * target_rotation));
            is_cursor_selecting = true;
        }
        _ => (),
    }

    cursor_command.command_type = command_type;
    cursor_command.cursor_direction_camera_frame = Some(cursor_direction_camera_frame);
    cursor_command.cursor_selection = if is_cursor_selecting {
        Some(cursor_selection)
    } else {
        None
    };

    return cursor_command;
}

// Returns the object selected by the cursor, if none, defaults to ground plane or arbitrary point in front
fn get_cursor_selected_point(cursor_raycast_source: &RaycastSource<SiteRaycastSet>) -> Vec3 {
    let cursor_ray = cursor_raycast_source.get_ray().unwrap();

    match cursor_raycast_source.get_nearest_intersection() {
        Some((_, intersection)) => intersection.position(),
        None => {
            // If valid intersection with groundplane
            let denom = Vec3::Z.dot(cursor_ray.direction());
            if denom.abs() > f32::EPSILON {
                let dist = (-1.0 * cursor_ray.origin()).dot(Vec3::Z) / denom;
                if dist > f32::EPSILON && dist < MAX_SELECTION_DIST {
                    return cursor_ray.origin() + cursor_ray.direction() * dist;
                }
            }

            // No groundplane intersection
            // Pick a point of a virtual sphere around the camera, of same radius as its height
            let height = cursor_ray.origin().z.abs();
            let radius = if height < 1.0 { 1.0 } else { height };
            return cursor_ray.origin() + cursor_ray.direction() * radius;
        }
    }
}

fn get_command_type(
    keyboard_input: &Res<Input<KeyCode>>,
    mouse_input: &Res<Input<MouseButton>>,
    cursor_motion: &Vec2,
    scroll_motion: &f32,
    projection_mode: ProjectionMode,
    command_type_prev: &CameraCommandType,
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
        || (is_shifting && mouse_input.pressed(MouseButton::Right))
    {
        return CameraCommandType::Orbit;
    } else if is_shifting
        && (*command_type_prev == CameraCommandType::Orbit
            || *command_type_prev == CameraCommandType::HoldOrbitSelection)
        && projection_mode.is_perspective()
    {
        return CameraCommandType::HoldOrbitSelection;
    }

    // Zoom
    if projection_mode.is_orthographic() && is_scrolling {
        return CameraCommandType::ScaleZoom;
    }
    if projection_mode.is_perspective() && is_scrolling {
        if is_shifting {
            return CameraCommandType::FovZoom;
        } else {
            return CameraCommandType::TranslationZoom;
        }
    }

    return CameraCommandType::Inactive;
}
