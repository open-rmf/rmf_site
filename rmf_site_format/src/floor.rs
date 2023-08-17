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

use crate::*;
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Entity};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Floor<T: RefTrait> {
    pub anchors: Path<T>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub texture: Texture,
    #[serde(
        default = "PreferredSemiTransparency::for_floor",
        skip_serializing_if = "PreferredSemiTransparency::is_default_for_floor"
    )]
    pub preferred_semi_transparency: PreferredSemiTransparency,
    #[serde(skip)]
    pub marker: FloorMarker,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct FloorMarker;

#[cfg(feature = "bevy")]
impl Floor<Entity> {
    pub fn to_u32(&self, anchors: Path<u32>) -> Floor<u32> {
        Floor {
            anchors,
            texture: self.texture.clone(),
            preferred_semi_transparency: PreferredSemiTransparency::for_floor(),
            marker: Default::default(),
        }
    }
}

#[cfg(feature = "bevy")]
impl Floor<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Floor<Entity> {
        Floor {
            anchors: self.anchors.to_ecs(id_to_entity),
            texture: self.texture.clone(),
            preferred_semi_transparency: PreferredSemiTransparency::for_floor(),
            marker: Default::default(),
        }
    }
}

impl<T: RefTrait> From<Path<T>> for Floor<T> {
    fn from(path: Path<T>) -> Self {
        Floor {
            anchors: path,
            texture: Default::default(),
            preferred_semi_transparency: PreferredSemiTransparency::for_floor(),
            marker: Default::default(),
        }
    }
}
