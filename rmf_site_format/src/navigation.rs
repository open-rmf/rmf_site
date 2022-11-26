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
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Navigation {
    #[serde(default, skip_serializing_if="Guided::is_empty")]
    pub guided: Guided,
}

impl Navigation {
    pub fn is_empty(&self) -> bool {
        self.guided.is_empty()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Guided {
    /// Properties of each nav graph
    pub graphs: BTreeMap<u32, NavGraph>,
    /// Properties of each robot traffic lane
    pub lanes: BTreeMap<u32, Lane<u32>>,
    /// Properties of each special location
    pub locations: BTreeMap<u32, Location<u32>>,
}

impl Guided {
    pub fn is_empty(&self) -> bool {
        self.graphs.is_empty() && self.lanes.is_empty() && self.locations.is_empty()
    }
}
