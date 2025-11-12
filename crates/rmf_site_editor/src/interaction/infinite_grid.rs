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

use crate::{
    interaction::{Hovered, Selected},
    site::{
        as_minor_line_color, HOVER_COLOR, HOVER_SELECT_COLOR, SELECT_COLOR,
        STANDARD_GRID_LINE_COLOR,
    },
};
use bevy::prelude::*;
use bevy_infinite_grid::InfiniteGridSettings;

pub fn update_infinite_grid_cues(
    mut changed_grid: Query<
        (&Hovered, &Selected, &mut InfiniteGridSettings),
        Or<(Changed<Hovered>, Changed<Selected>)>,
    >,
) {
    for (hovered, selected, mut grid) in &mut changed_grid {
        let color = if hovered.cue() && selected.cue() {
            HOVER_SELECT_COLOR
        } else if hovered.cue() {
            HOVER_COLOR
        } else if selected.cue() {
            SELECT_COLOR
        } else {
            STANDARD_GRID_LINE_COLOR
        };

        grid.major_line_color = color;
        grid.minor_line_color = as_minor_line_color(color);
    }
}
