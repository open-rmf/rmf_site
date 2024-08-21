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

use bevy::prelude::*;
use bevy_mod_raycast::{
    immediate::{Raycast, RaycastSettings, RaycastVisibility},
    primitives::Ray3d,
};

use super::{MAX_PITCH, MAX_SELECTION_DIST, MIN_SELECTION_DIST};
use crate::UserCameraDisplay;

/// Orbit the camera around the point while upright, maintaining the orbit center
/// in the same position in the camera viewport
pub fn orbit_camera_around_point(
    camera_transform: &Transform,
    orbit_center: Vec3,
    pitch: f32,
    yaw: f32,
    zoom: f32,
) -> Transform {
    let mut camera_transform_next = camera_transform.clone();

    // Rotation
    // Exclude pitch if exceeds maximum angle in orthographic mode
    let yaw = Quat::from_rotation_z(yaw);
    let pitch = Quat::from_rotation_x(-pitch);
    camera_transform_next.rotation = (yaw * camera_transform.rotation) * pitch;
    if camera_transform_next.up().z.acos().to_degrees() > MAX_PITCH {
        camera_transform_next.rotation = yaw * camera_transform.rotation;
    };

    // Translation
    let camera_to_orbit_center = orbit_center - camera_transform.translation;
    let camera_to_orbit_center_camera_frame =
        camera_transform.rotation.inverse() * camera_to_orbit_center;
    let camera_to_orbit_center_next = camera_transform_next.local_x()
        * camera_to_orbit_center_camera_frame.x
        + camera_transform_next.local_y() * camera_to_orbit_center_camera_frame.y
        + camera_transform_next.local_z() * camera_to_orbit_center_camera_frame.z;
    camera_transform_next.translation = orbit_center - camera_to_orbit_center_next;

    // Zoom
    let zoom_translation = camera_to_orbit_center_next.normalize()
        * zoom_distance_factor(camera_transform.translation, orbit_center)
        * zoom;
    camera_transform_next.translation += zoom_translation;

    return camera_transform_next;
}

/// Multiplied over the zoom translation to make zoom proportionally faster when further away
pub fn zoom_distance_factor(camera_translation: Vec3, target_translation: Vec3) -> f32 {
    return (0.2 * (camera_translation - target_translation).length()).max(1.0);
}

/// Get the nearest intersection from the center of the visible viewport.
pub fn get_camera_selected_point(
    camera: &Camera,
    camera_global_transform: &GlobalTransform,
    user_camera_display: Res<UserCameraDisplay>,
    mut immediate_raycast: Raycast,
) -> Option<Vec3> {
    // Assume that the camera spans the full window, covered by egui panels
    let available_viewport_center = user_camera_display.region.center();
    let camera_ray =
        camera.viewport_to_world(camera_global_transform, available_viewport_center)?;
    let camera_ray = Ray3d::new(camera_ray.origin, camera_ray.direction);
    let raycast_setting = RaycastSettings::default()
        .always_early_exit()
        .with_visibility(RaycastVisibility::MustBeVisible);

    //TODO(@reuben-thomas) Filter for selectable entities
    let intersections = immediate_raycast.cast_ray(camera_ray, &raycast_setting);
    if intersections.len() > 0 {
        let (_, intersection_data) = &intersections[0];
        return Some(intersection_data.position());
    } else {
        return Some(get_groundplane_else_default_selection(
            camera_ray.origin(),
            camera_ray.direction(),
            camera_ray.direction(),
        ));
    }
}

/// Get the intersection point of a ray with the ground. If none, return an intersection with
/// a plane in front of the camera.
pub fn get_groundplane_else_default_selection(
    selector_origin: Vec3,
    selector_direction: Vec3,
    camera_direction: Vec3,
) -> Vec3 {
    // If valid intersection with groundplane
    let denom = Vec3::Z.dot(selector_direction);
    if denom.abs() > f32::EPSILON {
        let dist = (-1.0 * selector_origin).dot(Vec3::Z) / denom;
        if dist > f32::EPSILON && dist < MAX_SELECTION_DIST {
            return selector_origin + selector_direction * dist;
        }
    }

    // No groundplane intersection,
    // Pick a point on arbitrary plane in front
    let height = selector_origin.z.abs();
    let plane_dist = height.max(MIN_SELECTION_DIST);
    return selector_origin
        + selector_direction * (plane_dist / selector_direction.dot(camera_direction));
}
