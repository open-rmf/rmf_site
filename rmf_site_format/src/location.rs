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
use bevy::prelude::{Component, Bundle, Entity, Deref, DerefMut};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LocationTag {
    Charger,
    ParkingSpot,
    HoldingPoint,
    SpawnRobot(Model),
    Workcell(Model),
    Name(String),
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature="bevy", derive(Bundle))]
pub struct Location<T: SiteID> {
    pub anchor: Point<T>,
    pub tags: LocationTags,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature="bevy", derive(Component, Deref, DerefMut))]
pub struct LocationTags(pub Vec<LocationTag>);

impl Default for LocationTags {
    fn default() -> Self {
        LocationTags(Vec::new())
    }
}

#[cfg(feature="bevy")]
impl Location<Entity> {
    pub fn to_u32(&self, anchor: u32) -> Location<u32> {
        Location{
            anchor: Point(anchor),
            tags: self.tags.clone(),
        }
    }
}

#[cfg(feature="bevy")]
impl Location<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Location<Entity> {
        Location{
            anchor: Point(*id_to_entity.get(&self.anchor).unwrap()),
            tags: self.tags.clone(),
        }
    }
}

impl<T: SiteID> From<Point<T>> for Location<T> {
    fn from(anchor: Point<T>) -> Self {
        Self{anchor, tags: Default::default()}
    }
}
