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
use bevy::prelude::{Bundle, Component, Entity, Reflect, ReflectComponent};
use serde::{Deserialize, Serialize};
use bevy::platform::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
#[cfg_attr(feature = "bevy", derive(Reflect))]
pub struct Wall {
    pub anchors: Edge,
    #[serde(skip_serializing_if = "is_default")]
    pub texture: Affiliation,
    #[serde(skip)]
    pub marker: WallMarker,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct WallMarker;

impl Wall {
    pub fn convert(&self, id_map: &HashMap<Entity, Entity>) -> Result<Wall, Entity> {
        Ok(Wall {
            anchors: self.anchors.convert(id_map)?,
            texture: self.texture.convert(id_map)?,
            marker: Default::default(),
        })
    }
}

impl From<Edge> for Wall {
    fn from(anchors: Edge) -> Self {
        Self {
            anchors,
            texture: Affiliation(None),
            marker: Default::default(),
        }
    }
}
