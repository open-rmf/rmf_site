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

use bevy::prelude::*;

#[derive(Component, Clone, Copy, Debug)]
pub struct PathBehavior {
    /// Whether the path is allowed to have an inner loop. E.g.
    /// `A -> B -> C -> D -> B` would be an inner loop.
    pub allow_inner_loops: bool,
    /// A minimum for how many points need to be selected for the path to be
    /// considered valid. Use 0 if there is no minimum.
    pub minimum_points: usize,
    /// The path is implied to always be a complete loop. This has two consequences:
    /// 1. If the first point gets re-selected later in the path then we automatically
    ///    consider the path to be finished.
    /// 2. When the (1) occurs, the first point does not get re-added to the path.
    pub implied_complete_loop: bool,
}

impl PathBehavior {
    pub fn for_floor() -> Self {
        Self{
            allow_inner_loops: false,
            minimum_points: 3,
            implied_complete_loop: true,
        }
    }
}
