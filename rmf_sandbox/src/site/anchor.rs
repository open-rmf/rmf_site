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

use std::collections::HashSet;
use bevy::prelude::*;

#[derive(Component)]
pub struct Anchor(f32, f32);

impl Anchor {
    pub fn vec(&self) -> Vec2 {
        Vec2::new(self.0, self.1)
    }

    pub fn x(&self) -> f32 {
        self.0
    }

    pub fn y(&self) -> f32 {
        self.1
    }

    pub fn transform(&self) -> Transform {
        Transform::from_xyz(self.0, self.1, 0.0)
    }
}

#[derive(Component)]
pub struct AnchorDependents {
    pub dependents: HashSet<Entity>,
}
