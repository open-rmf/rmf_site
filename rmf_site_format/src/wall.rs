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
use serde::{Serialize, Deserialize};
#[cfg(feature="bevy")]
use bevy::prelude::{Component, Entity};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct Wall<SiteID> {
    pub anchors: (SiteID, SiteID),
    #[serde(skip_serializing_if="Option::is_none")]
    pub texture: Option<Texture>,
}

#[cfg(feature="bevy")]
impl Wall<Entity> {
    pub fn to_u32(&self, anchors: (u32, u32)) -> Wall<u32> {
        Wall{
            anchors,
            texture: self.texture.clone(),
        }
    }
}

#[cfg(feature="bevy")]
impl Wall<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Wall<Entity> {
        Wall{
            anchors: (
                *id_to_entity.get(&self.anchors.0).unwrap(),
                *id_to_entity.get(&self.anchors.1).unwrap(),
            ),
            texture: self.texture.clone(),
        }
    }
}

impl<SiteID: Copy> Edge<SiteID> for Wall<SiteID> {
    fn endpoints(&self) -> (SiteID, SiteID) {
        self.anchors
    }

    fn endpoints_mut(&mut self) -> (&mut SiteID, &mut SiteID) {
        (&mut self.anchors.0, &mut self.anchors.1)
    }
}
