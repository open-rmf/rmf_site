/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use bevy::prelude::{ChildOf, Color, Commands, Deref, Entity, Quat, Resource, Transform, Vec3};
use bevy_infinite_grid::{InfiniteGridBundle, InfiniteGridSettings};

use rmf_site_picking::{Hovered, Selected};

#[derive(Resource, Clone, Copy, Deref)]
pub struct ReferenceGrid(Entity);

pub const STANDARD_GRID_LINE_COLOR: Color = Color::srgb(0.4, 0.4, 0.4);
pub fn as_minor_line_color(major_color: Color) -> Color {
    let major = major_color.to_srgba();
    Color::srgba(
        major.red / 2.0,
        major.green / 2.0,
        major.blue / 2.0,
        major.alpha,
    )
}

pub fn spawn_standard_infinite_grid(commands: &mut Commands, site: Entity) {
    let grid = commands
        .spawn((
            InfiniteGridBundle {
                transform: Transform {
                    translation: Vec3::new(0., 0., -0.01),
                    rotation: Quat::from_rotation_x(90_f32.to_radians()),
                    scale: Vec3::splat(1.0),
                },
                settings: InfiniteGridSettings {
                    minor_line_color: as_minor_line_color(STANDARD_GRID_LINE_COLOR),
                    major_line_color: STANDARD_GRID_LINE_COLOR,
                    ..Default::default()
                },
                ..Default::default()
            },
            ChildOf(site),
            Hovered::default(),
            Selected::default(),
        ))
        .id();

    commands.insert_resource(ReferenceGrid(grid));
}
