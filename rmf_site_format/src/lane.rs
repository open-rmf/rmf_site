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
use bevy::prelude::Component;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct Lane<SiteID> {
    /// The endpoints of the lane (start, end)
    pub anchors: (SiteID, SiteID),
    /// The properties of the lane when traveling forwards
    pub forward: Motion,
    /// The properties of the lane when traveling in reverse
    pub reverse: ReverseLane,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Motion {
    #[serde(skip_serializing_if="Option::is_none")]
    pub orientation_constraint: Option<OrientationConstraint>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub speed_limit: Option<f32>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub dock: Option<Dock>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum OrientationConstraint {
    Forward,
    Reverse,
    RelativeYaw(f32),
    AbsoluteYaw(f32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum ReverseLane {
    Same,
    Disable,
    Different(Motion),
}

#[cfg(feature="bevy")]
impl<SiteID> Lane<SiteID> {
    pub fn to_u32(&self, anchors: (u32, u32)) -> Lane<u32> {
        Lane{
            anchors,
            ..self.clone()
        }
    }
}
