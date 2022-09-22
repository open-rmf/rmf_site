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

use crate::{Side, RefTrait};
use serde::{Serialize, Deserialize};
#[cfg(feature="bevy")]
use bevy::prelude::{Component, Entity};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct Edge<T>([T; 2]);

impl<T: RefTrait> Edge<T> {
    /// Create a new edge of this type using the given anchors. All other
    /// properties of the edge should have sensible default values.
    pub fn new(left: T, right: T) -> Self {
        Self([left, right])
    }

    pub fn array(&self) -> [T; 2] {
        self.0
    }

    pub fn array_mut(&mut self) -> &mut [T; 2] {
        &mut self.0
    }

    pub fn left(&self) -> T {
        self.0[0]
    }

    pub fn left_mut(&mut self) -> &mut T {
        self.0.get_mut(0).unwrap()
    }

    pub fn right(&self) -> T {
        self.0[1]
    }

    pub fn right_mut(&mut self) -> &mut T {
        self.0.get_mut(1).unwrap()
    }

    pub fn start(&self) -> T {
        self.left()
    }

    pub fn start_mut(&mut self) -> &mut T {
        self.left_mut()
    }

    pub fn end(&self) -> T {
        self.right()
    }

    pub fn end_mut(&mut self) -> &mut T {
        self.right_mut()
    }

    pub fn side(&self, side: Side) -> T {
        match side {
            Side::Left => self.left(),
            Side::Right => self.right(),
        }
    }
}

impl<T: RefTrait> From<[T; 2]> for Edge<T> {
    fn from(array: [T; 2]) -> Self {
        Self(array)
    }
}

#[cfg(feature="bevy")]
impl Edge<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Edge<Entity> {
        Edge([
            *id_to_entity.get(&self.left()).unwrap(),
            *id_to_entity.get(&self.right()).unwrap(),
        ])
    }
}
