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

use crate::SiteID;
#[cfg(feature = "bevy")]
use bevy::prelude::{Deref, DerefMut};
use bevy_ecs::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Component, Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Deref, DerefMut))]
pub struct Point(pub SiteID);

impl From<SiteID> for Point {
    fn from(anchor: SiteID) -> Self {
        Self(anchor)
    }
}

impl Point {
    pub fn convert(&self, id_map: &HashMap<SiteID, Entity>) -> Result<Point, SiteID> {
        Ok(Point(id_map.get(&self.0).ok_or(self.0)?.clone().into()))
    }
}
