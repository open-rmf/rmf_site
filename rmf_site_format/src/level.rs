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
use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};
#[cfg(feature="bevy")]
use bevy::prelude::Component;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct LevelProperties {
    pub name: String,
    pub elevation: f32,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Level {
    pub properties: LevelProperties,
    pub anchors: BTreeMap<u32, (f32, f32)>,
    pub doors: BTreeMap<u32, Door<u32>>,
    pub drawings: BTreeMap<u32, Drawing>,
    pub fiducials: BTreeMap<u32, Fiducial<u32>>,
    pub floors: BTreeMap<u32, Floor<u32>>,
    pub lights: BTreeMap<u32, Light>,
    pub measurements: BTreeMap<u32, Measurement<u32>>,
    pub models: BTreeMap<u32, Model>,
    pub physical_cameras: BTreeMap<u32, PhysicalCamera>,
    pub walls: BTreeMap<u32, Wall<u32>>,
}

impl Level {
    pub fn new(properties: LevelProperties) -> Level {
        Level{
            properties,
            anchors: Default::default(),
            doors: Default::default(),
            drawings: Default::default(),
            fiducials: Default::default(),
            floors: Default::default(),
            lights: Default::default(),
            measurements: Default::default(),
            models: Default::default(),
            physical_cameras: Default::default(),
            walls: Default::default(),
        }
    }
}
