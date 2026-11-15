/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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

pub fn draw_crosshair_symbol(
    gizmos: &mut Gizmos,
    pos: Vec3,
    rotation: Quat,
    outer_radius: f32,
    inner_radius: f32,
    cross_radius: f32,
    color: Color,
) {
    let isometry_pos = Isometry3d::new(pos, rotation);
    gizmos.circle(isometry_pos, outer_radius, color);
    gizmos.circle(isometry_pos, inner_radius, color);

    let x_axis = rotation * Vec3::X * cross_radius;
    let y_axis = rotation * Vec3::Y * cross_radius;
    gizmos.line(pos + x_axis, pos - x_axis, color);
    gizmos.line(pos + y_axis, pos - y_axis, color);
}

pub fn draw_off_limits_symbol(
    gizmos: &mut Gizmos,
    pos: Vec3,
    rotation: Quat,
    radius: f32,
    color: Color,
) {
    let isometry_pos = Isometry3d::new(pos, rotation);
    gizmos.circle(isometry_pos, radius, color);

    let slash = rotation * Vec3::new(-1.0, 1.0, 0.0).normalize() * radius;
    gizmos.line(pos + slash, pos - slash, color);
}

pub fn draw_dashed_line(
    gizmos: &mut Gizmos,
    start_pos: Vec3,
    end_pos: Vec3,
    dash_length: f32,
    gap_length: f32,
    phase_offset: f32,
    color: Color,
) {
    let delta = end_pos - start_pos;
    let distance = delta.length();
    if distance <= 0.0 {
        return;
    }

    let dir = delta / distance;
    let pattern_length = dash_length + gap_length;
    let offset = phase_offset.rem_euclid(pattern_length);
    let mut current_dist = if offset > 0.0 {
        offset - pattern_length
    } else {
        0.0
    };

    while current_dist < distance {
        let start_dist = current_dist.max(0.0);
        let end_dist = (current_dist + dash_length).min(distance);
        if start_dist < end_dist {
            gizmos.line(
                start_pos + dir * start_dist,
                start_pos + dir * end_dist,
                color,
            );
        }
        current_dist += pattern_length;
    }
}

pub fn draw_path_arrowhead(
    gizmos: &mut Gizmos,
    pos: Vec3,
    dir: Vec3,
    arrow_size: f32,
    color: Color,
) {
    let perp = Vec3::new(-dir.y, dir.x, 0.0);
    let p1 = pos - dir * arrow_size + perp * (arrow_size * 0.5);
    let p2 = pos - dir * arrow_size - perp * (arrow_size * 0.5);

    gizmos.line(pos, p1, color);
    gizmos.line(pos, p2, color);
}

pub fn draw_path_waiting_point(
    gizmos: &mut Gizmos,
    pos: Vec3,
    outer_radius: f32,
    inner_radius: f32,
    color: Color,
) {
    let isometry_pos = Isometry3d::new(pos, Quat::IDENTITY);
    gizmos.circle(isometry_pos, outer_radius, color);
    gizmos.circle(isometry_pos, inner_radius, color);
}

pub fn draw_path_start_point(
    gizmos: &mut Gizmos,
    pos: Vec3,
    inner_radius: f32,
    cross_radius: f32,
    color: Color,
) {
    let isometry_pos = Isometry3d::new(pos, Quat::IDENTITY);
    gizmos.circle(isometry_pos, inner_radius, color);
    gizmos.line(
        pos + Vec3::X * cross_radius,
        pos - Vec3::X * cross_radius,
        color,
    );
    gizmos.line(
        pos + Vec3::Y * cross_radius,
        pos - Vec3::Y * cross_radius,
        color,
    );
}
