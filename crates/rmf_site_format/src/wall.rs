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
use bevy_ecs::prelude::{Bundle, Component, Entity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Bundle, Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Wall {
    pub anchors: Edge,
    #[serde(skip_serializing_if = "is_default")]
    pub texture: Affiliation,
    #[serde(skip)]
    pub marker: WallMarker,
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq)]
pub struct WallMarker;

impl Wall {
    pub fn convert(&self, id_map: &HashMap<SiteID, Entity>) -> Result<Wall, SiteID> {
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
