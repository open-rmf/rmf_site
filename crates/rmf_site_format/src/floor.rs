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
pub struct Floor {
    pub anchors: Path,
    #[serde(default, skip_serializing_if = "is_default")]
    pub texture: Affiliation,
    #[serde(
        default = "PreferredSemiTransparency::for_floor",
        skip_serializing_if = "PreferredSemiTransparency::is_default_for_floor"
    )]
    pub preferred_semi_transparency: PreferredSemiTransparency,
    #[serde(skip)]
    pub marker: FloorMarker,
}

#[derive(Component, Clone, Copy, Debug, Default, PartialEq, Eq)]
pub struct FloorMarker;

impl Floor {
    pub fn convert(&self, id_map: &HashMap<SiteID, Entity>) -> Result<Floor, SiteID> {
        Ok(Floor {
            anchors: self.anchors.convert(id_map)?,
            texture: self.texture.convert(id_map)?,
            preferred_semi_transparency: PreferredSemiTransparency::for_floor(),
            marker: Default::default(),
        })
    }
}

impl From<Path> for Floor {
    fn from(path: Path) -> Self {
        Floor {
            anchors: path,
            texture: Affiliation(None),
            preferred_semi_transparency: PreferredSemiTransparency::for_floor(),
            marker: Default::default(),
        }
    }
}
