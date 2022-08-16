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

use serde::{Serialize, Deserialize};
#[cfg(feature="bevy")]
use bevy::prelude::Component;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct Measurement<SiteID> {
    pub anchors: (SiteID, SiteID),
    pub distance: f32,
    pub label: String,
}

#[cfg(feature="bevy")]
impl<SiteID> Measurement<SiteID> {
    pub fn to_u32(&self, anchors: (u32, u32)) -> Measurement<u32> {
        Measurement{
            anchors,
            distance: self.distance,
            label: self.label.clone()
        }
    }
}
