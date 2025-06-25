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
    components::{OrthographicCameraRoot, PerspectiveCameraRoot},
    resources::CameraConfig,
};

use super::{
    CameraCommandType, MAX_FOV, MAX_SCALE, MIN_FOV, MIN_SCALE, ProjectionMode,
    get_groundplane_else_default_selection, orbit_camera_around_point, zoom_distance_factor,
};
use bevy_ecs::prelude::*;
use bevy_input::{
    mouse::{MouseScrollUnit, MouseWheel},
    prelude::*,
};
use bevy_math::prelude::*;
use bevy_picking::{
    backend::ray::RayMap,
    pointer::{PointerId, PointerInteraction},
};
use bevy_reflect::Reflect;
use bevy_render::prelude::*;
use bevy_transform::components::{GlobalTransform, Transform};
use bevy_window::{PrimaryWindow, Window};
use nalgebra::{Matrix3, Matrix3x1};
use tracing::warn;

pub const SCALE_ZOOM_SENSITIVITY: f32 = 0.1;
pub const TRANSLATION_ZOOM_SENSITIVITY: f32 = 0.2;
pub const FOV_ZOOM_SENSITIVITY: f32 = 0.1;

/// Current cursor command for active camera.
#[derive(Resource, Reflect)]
#[reflect(Resource)]
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

impl CursorCommand {
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

pub fn update_cursor_command(
    mut camera_config: ResMut<CameraConfig>,
    mut cursor_command: ResMut<CursorCommand>,
    mut mouse_wheel: EventReader<MouseWheel>,
    mouse_input: Res<ButtonInput<MouseButton>>,
    projection_mode: Res<ProjectionMode>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
    pointers: Query<(&PointerId, &PointerInteraction)>,
    ortho_cam: Query<Entity, With<OrthographicCameraRoot>>,
    persp_cam: Query<Entity, With<PerspectiveCameraRoot>>,
    ray_map: Res<RayMap>,
    cameras: Query<(&Projection, &Transform, &GlobalTransform)>,
    primary_windows: Query<&Window, With<PrimaryWindow>>,
) {
    if let Ok(window) = primary_windows.single() {
        // Return if cursor not within window
        if window.cursor_position().is_none() {
            return;
        }

        // Scroll input
        let mut scroll_motion = 0.0;
        for ev in mouse_wheel.read() {
            scroll_motion += match ev.unit {
                MouseScrollUnit::Line => ev.y,
                MouseScrollUnit::Pixel => ev.y / 100.0,
            };
        }

        // Command type, return if inactive
        let command_type_prev = cursor_command.command_type;
        let command_type = get_command_type(
            &keyboard_input,
            &mouse_input,
            &scroll_motion,
            *projection_mode,
        );
        if command_type == CameraCommandType::Inactive {
            *cursor_command = CursorCommand::default();
            return;
        }

        // Camera projection and transform
        let active_camera_entity = match *projection_mode {
            ProjectionMode::Orthographic => ortho_cam.single(),
            ProjectionMode::Perspective => persp_cam.single(),
        };

        let Ok(active_camera_entity) = active_camera_entity
            .inspect_err(|err| warn!("could not update cursor command due to {:#}", err))
        else {
            return;
        };

        let (camera_proj, camera_transform, _) = cameras.get(active_camera_entity).unwrap();

        // Get selection under cursor, cursor direction
        let Some((_, cursor_ray)) = ray_map
            .iter()
            .find(|(id, _)| id.camera == active_camera_entity)
        else {
            return;
        };

        let cursor_selection_new = pointers
            .single()
            .ok()
            .and_then(|(_, interactions)| {
                interactions
                    .iter()
                    .find(|(_, hit)| hit.camera == active_camera_entity)
            })
            .and_then(|(_, hit_data)| hit_data.position)
            .unwrap_or_else(|| {
                get_groundplane_else_default_selection(
                    cursor_ray.origin,
                    *cursor_ray.direction,
                    *camera_transform.forward(),
                )
            });
        let cursor_selection = match cursor_command.cursor_selection {
            Some(selection) => selection,
            None => cursor_selection_new,
        };

        let cursor_direction = cursor_ray.direction.normalize();
        let cursor_direction_camera_frame = camera_transform.rotation.inverse() * cursor_direction;
        let cursor_direction_camera_frame_prev = cursor_command
            .cursor_direction_camera_frame
            .unwrap_or(cursor_direction_camera_frame);

        // Update orbit center
        if command_type != CameraCommandType::Orbit {
            camera_config.orbit_center = None;
        } else if (command_type == CameraCommandType::Orbit && command_type != command_type_prev)
            || camera_config.orbit_center.is_none()
        {
            camera_config.orbit_center = Some(cursor_selection);
        }
        let orbit_center = camera_config.orbit_center.unwrap_or(cursor_selection);

        match *projection_mode {
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
                        orbit_center,
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
    let target_scale = (camera_proj.scale
        - scroll_motion * camera_proj.scale * SCALE_ZOOM_SENSITIVITY)
        .clamp(MIN_SCALE, MAX_SCALE);
    cursor_command.scale_delta = target_scale - camera_proj.scale;

    //TODO(@reuben-thomas) Find out why cursor ray cannot be used for direction
    let cursor_direction = (cursor_selection_new - camera_transform.translation).normalize();

    match command_type {
        CameraCommandType::Pan => {
            let camera_transform_next = pan_camera_with_cursor(
                camera_transform,
                cursor_selection,
                cursor_direction,
                scroll_motion,
            );
            cursor_command.translation_delta =
                camera_transform_next.translation - camera_transform.translation;
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
    orbit_center: Vec3,
) -> CursorCommand {
    let mut cursor_command = CursorCommand::default();
    let mut is_cursor_selecting = false;

    match command_type {
        CameraCommandType::FovZoom => {
            let target_fov = (camera_proj.fov - scroll_motion * FOV_ZOOM_SENSITIVITY)
                .clamp(MIN_FOV.to_radians(), MAX_FOV.to_radians());
            cursor_command.fov_delta = target_fov - camera_proj.fov;
        }
        CameraCommandType::TranslationZoom => {
            cursor_command.translation_delta = cursor_direction
                * zoom_distance_factor(camera_transform.translation, cursor_selection)
                * TRANSLATION_ZOOM_SENSITIVITY
                * scroll_motion
        }
        CameraCommandType::Pan => {
            let camera_transform_next = pan_camera_with_cursor(
                camera_transform,
                cursor_selection,
                cursor_direction,
                scroll_motion,
            );
            cursor_command.translation_delta =
                camera_transform_next.translation - camera_transform.translation;
            is_cursor_selecting = true;
        }
        CameraCommandType::Orbit => {
            // Pitch and yaw inputs from cursor direction vector as spherical coordinates
            let pitch = cursor_direction_camera_frame_prev.y.acos()
                - cursor_direction_camera_frame.y.acos();
            let yaw = (cursor_direction_camera_frame_prev
                .z
                .atan2(cursor_direction_camera_frame_prev.x))
                - (cursor_direction_camera_frame
                    .z
                    .atan2(cursor_direction_camera_frame.x));

            // Scale inputs to viewport range
            let pitch_viewport_range = camera_proj.fov;
            let pitch = -pitch / pitch_viewport_range * std::f32::consts::PI;
            let yaw_viewport_range = camera_proj.fov * camera_proj.aspect_ratio;
            let yaw = yaw / yaw_viewport_range * 2.0 * std::f32::consts::PI;
            let zoom = TRANSLATION_ZOOM_SENSITIVITY * scroll_motion;

            let camera_transform_next =
                orbit_camera_around_point(camera_transform, orbit_center, pitch, yaw, zoom);

            cursor_command.translation_delta =
                camera_transform_next.translation - camera_transform.translation;
            cursor_command.rotation_delta =
                camera_transform.rotation.inverse() * camera_transform_next.rotation;
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

/// Pans camera such that selection remains under cursor
fn pan_camera_with_cursor(
    camera_transform: &Transform,
    cursor_selection: Vec3,
    cursor_direction: Vec3,
    scroll_motion: f32,
) -> Transform {
    let mut camera_transform_next = camera_transform.clone();
    // To keep the same point below the cursor, we solve
    // selection_to_camera + translation_delta = selection_to_camera_next
    // selection_to_camera_next = x3 * -cursor_direction
    // translation_delta = x1 * right_ transltion + x2 * up_translation
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
    camera_transform_next.translation += x[0] * right_translation + x[1] * up_translation;

    let zoom_translation = cursor_direction
        * zoom_distance_factor(
            camera_transform.translation,
            camera_transform_next.translation,
        )
        * TRANSLATION_ZOOM_SENSITIVITY
        * scroll_motion;
    camera_transform_next.translation += zoom_translation;

    return camera_transform_next;
}

fn get_command_type(
    keyboard_input: &Res<ButtonInput<KeyCode>>,
    mouse_input: &Res<ButtonInput<MouseButton>>,
    scroll_motion: &f32,
    projection_mode: ProjectionMode,
) -> CameraCommandType {
    // Inputs
    let is_scrolling = *scroll_motion != 0.;
    let is_shifting =
        keyboard_input.pressed(KeyCode::ShiftLeft) || keyboard_input.pressed(KeyCode::ShiftRight);

    // Panning
    if !is_shifting && mouse_input.pressed(MouseButton::Right) {
        return CameraCommandType::Pan;
    }

    // Orbitting
    if mouse_input.pressed(MouseButton::Middle)
        || (is_shifting && mouse_input.pressed(MouseButton::Right))
    {
        return CameraCommandType::Orbit;
    }

    // Zoom
    if projection_mode == ProjectionMode::Orthographic && is_scrolling {
        return CameraCommandType::ScaleZoom;
    }
    if projection_mode == ProjectionMode::Perspective && is_scrolling {
        if is_shifting {
            return CameraCommandType::FovZoom;
        } else {
            return CameraCommandType::TranslationZoom;
        }
    }

    return CameraCommandType::Inactive;
}
