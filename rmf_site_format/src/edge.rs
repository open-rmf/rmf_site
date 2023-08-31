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

use crate::{RefTrait, Side};
#[cfg(feature = "bevy")]
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component))]
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

    pub fn side_mut(&mut self, side: Side) -> &mut T {
        match side {
            Side::Left => self.left_mut(),
            Side::Right => self.right_mut(),
        }
    }

    pub fn with_side_of(mut self, side: Side, value: T) -> Self {
        *self.side_mut(side) = value;
        self
    }

    pub fn in_reverse(&self) -> Self {
        Self([self.right(), self.left()])
    }

    pub fn is_reverse_of(&self, other: &Self) -> bool {
        self.left() == other.right() && self.right() == other.left()
    }
}

impl<T: RefTrait> From<[T; 2]> for Edge<T> {
    fn from(array: [T; 2]) -> Self {
        Self(array)
    }
}

impl<T: RefTrait> Edge<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Edge<U>, T> {
        Ok(Edge([
            id_map.get(&self.left()).ok_or(self.left())?.clone(),
            id_map.get(&self.right()).ok_or(self.right())?.clone(),
        ]))
    }
}
