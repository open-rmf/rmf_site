/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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
use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct ScenarioProperties {
    pub name: String,
}

impl Default for ScenarioProperties {
    fn default() -> Self {
        Self {
            name: "new_scenario".to_owned(),
        }
    }
}

#[derive(Serailize, Deserialize, Debug, Clone, Default)]
pub struct Scenario {
    /// Basic properties of the scenario
    pub properties: ScenarioProperties,
    /// What mobile robots exist in the scenario
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub mobile_robots: BTreeMap<u32, MobileRobot>,
    /// What agents exist in the scenario
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub agents: BTreeMap<u32, Agent>,
}
