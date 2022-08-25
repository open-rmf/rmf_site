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
use bevy::prelude::{Component, Entity};

/// Mark a point within the map of a level to serve as a ground truth relative
/// to other levels.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct Fiducial<SiteID> {
    /// Label of this fiducial. This label must be unique within the level that
    /// the fiducial is being defined on. To be used for aligning, there must
    /// be a fiducial with the same label on one or more other levels.
    pub label: String,
    /// The anchor that represents the position of this fiducial.
    pub anchor: SiteID,
}

#[cfg(feature="bevy")]
impl Fiducial<Entity> {
    pub fn to_u32(&self, anchor: u32) -> Fiducial<u32> {
        Fiducial{label: self.label.clone(), anchor}
    }
}

#[cfg(feature="bevy")]
impl Fiducial<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Fiducial<Entity> {
        Fiducial{
            anchor: *id_to_entity.get(&self.anchor).unwrap(),
            label: self.label.clone(),
        }
    }
}
