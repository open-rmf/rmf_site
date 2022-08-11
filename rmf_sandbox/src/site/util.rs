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

use crate::site::*;
use bevy::prelude::*;

pub fn line_stroke_transform(
    start: &Anchor,
    end: &Anchor,
) -> Transform {
    let p_start = start.vec();
    let p_end = end.vec();
    let dp = p_end - p_start;
    let length = dp.length();
    let width = LANE_WIDTH;

    let yaw = dp.y.atan2(dp.x);
    let center = (p_start + p_end)/2.0;
    Transform{
        translation: Vec3::new(center.x, center.y, 0.),
        rotation: Quat::from_rotation_z(yaw),
        scale: Vec3::new(length, width, 1.),
        ..default()
    }
}

pub const DEFAULT_LEVEL_HEIGHT: f32 = 3.0;