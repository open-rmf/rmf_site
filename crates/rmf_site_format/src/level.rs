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
use bevy::prelude::{Deref, DerefMut};
use bevy_ecs::prelude::{Bundle, Component};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Bundle, Serialize, Deserialize, Debug, Clone, PartialEq)]
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

#[derive(Component, Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Deref, DerefMut))]
pub struct LevelElevation(pub f32);

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Level {
    pub properties: LevelProperties,
    pub anchors: BTreeMap<SiteID, Anchor>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub doors: BTreeMap<SiteID, Door>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub drawings: BTreeMap<SiteID, Drawing>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub floors: BTreeMap<SiteID, Floor>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lights: BTreeMap<SiteID, Light>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub physical_cameras: BTreeMap<SiteID, PhysicalCamera>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub walls: BTreeMap<SiteID, Wall>,
    #[serde(default, skip_serializing_if = "RankingsInLevel::is_empty")]
    pub rankings: RankingsInLevel,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub user_camera_poses: BTreeMap<SiteID, UserCameraPose>,
}

impl Level {
    pub fn new(properties: LevelProperties, rankings: RankingsInLevel) -> Level {
        Level {
            properties,
            rankings,
            anchors: Default::default(),
            doors: Default::default(),
            drawings: Default::default(),
            floors: Default::default(),
            lights: Default::default(),
            physical_cameras: Default::default(),
            walls: Default::default(),
            user_camera_poses: Default::default(),
        }
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct RankingsInLevel {
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub floors: Vec<SiteID>,
    #[serde(default, skip_serializing_if = "Vec::is_empty")]
    pub drawings: Vec<SiteID>,
}

impl RankingsInLevel {
    pub fn is_empty(&self) -> bool {
        self.floors.is_empty() && self.drawings.is_empty()
    }
}
