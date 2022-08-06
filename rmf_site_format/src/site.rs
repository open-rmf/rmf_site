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

pub const CURRENT_VERSION: &str = "0.1";

pub struct Site {
    pub format_version: String,
    pub name: String,
    pub levels: BTreeMap<u32, Level>,
    pub lifts: BTreeMap<u32, Lift<u32>>,
    pub nav_graphs: BTreeMap<u32, NavGraph>,
    pub agents: BTreeMap<u32, Agent>,
}
