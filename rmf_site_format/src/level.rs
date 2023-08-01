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
use bevy::prelude::{Bundle, Component, Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct LevelProperties {
    pub name: NameInSite,
    pub elevation: LevelElevation,
    #[serde(default, skip_serializing_if = "is_default")]
    pub global_floor_visibility: GlobalFloorVisibility,
    #[serde(default, skip_serializing_if = "is_default")]
    pub global_drawing_visibility: GlobalDrawingVisibility,
}

impl Default for LevelProperties {
    fn default() -> Self {
        Self {
            name: NameInSite("<Unnamed>".to_owned()),
            elevation: LevelElevation(0.0),
            global_floor_visibility: Default::default(),
            global_drawing_visibility: Default::default(),
        }
    }
}


#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct LevelElevation(pub f32);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Level {
    pub properties: LevelProperties,
    pub anchors: BTreeMap<u32, Anchor>,
    /// Constraints to be used for drawing scaling alignment
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub constraints: BTreeMap<u32, Constraint<u32>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub doors: BTreeMap<u32, Door<u32>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub drawings: BTreeMap<u32, Drawing>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub floors: BTreeMap<u32, Floor<u32>>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lights: BTreeMap<u32, Light>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub models: BTreeMap<u32, Model>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub physical_cameras: BTreeMap<u32, PhysicalCamera>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub walls: BTreeMap<u32, Wall<u32>>,
    #[serde(default, skip_serializing_if = "RankingsInLevel::is_empty")]
    pub rankings: RankingsInLevel,
}

impl Level {
    pub fn new(properties: LevelProperties, rankings: RankingsInLevel) -> Level {
        Level {
            properties,
            rankings,
            anchors: Default::default(),
            constraints: Default::default(),
            doors: Default::default(),
            drawings: Default::default(),
            floors: Default::default(),
            lights: Default::default(),
            models: Default::default(),
            physical_cameras: Default::default(),
            walls: Default::default(),
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct RankingsInLevel {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub floors: Vec<u32>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drawings: Vec<u32>,
}

impl RankingsInLevel {
    pub fn is_empty(&self) -> bool {
        self.floors.is_empty() && self.drawings.is_empty()
    }
}
