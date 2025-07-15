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

use bevy_ecs::prelude::Entity;
use crate::{Side};
#[cfg(feature = "bevy")]
use bevy::prelude::{Reflect, ReflectComponent, Component};
use serde::{Deserialize, Serialize};
use bevy::platform::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct Edge(#[entities] [Entity; 2]);

impl Edge {
    /// Create a new edge of this type using the given anchors. All other
    /// properties of the edge should have sensible default values.
    pub fn new(left: Entity, right: Entity) -> Self {
        Self([left, right])
    }

    pub fn array(&self) -> [Entity; 2] {
        self.0
    }

    pub fn array_mut(&mut self) -> &mut [Entity; 2] {
        &mut self.0
    }

    pub fn left(&self) -> Entity {
        self.0[0]
    }

    pub fn left_mut(&mut self) -> &mut Entity {
        self.0.get_mut(0).unwrap()
    }

    pub fn right(&self) -> Entity {
        self.0[1]
    }

    pub fn right_mut(&mut self) -> &mut Entity {
        self.0.get_mut(1).unwrap()
    }

    pub fn start(&self) -> Entity {
        self.left()
    }

    pub fn start_mut(&mut self) -> &mut Entity {
        self.left_mut()
    }

    pub fn end(&self) -> Entity {
        self.right()
    }

    pub fn end_mut(&mut self) -> &mut Entity {
        self.right_mut()
    }

    pub fn side(&self, side: Side) -> Entity {
        match side {
            Side::Left => self.left(),
            Side::Right => self.right(),
        }
    }

    pub fn side_mut(&mut self, side: Side) -> &mut Entity {
        match side {
            Side::Left => self.left_mut(),
            Side::Right => self.right_mut(),
        }
    }

    pub fn with_side_of(mut self, side: Side, value: Entity) -> Self {
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

impl From<[Entity; 2]> for Edge {
    fn from(array: [Entity; 2]) -> Self {
        Self(array)
    }
}

impl Edge {
    pub fn convert(&self, id_map: &HashMap<Entity, Entity>) -> Result<Edge, Entity> {
        Ok(Edge([
            id_map.get(&self.left()).ok_or(self.left())?.clone(),
            id_map.get(&self.right()).ok_or(self.right())?.clone(),
        ]))
    }
}
