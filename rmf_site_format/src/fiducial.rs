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

use crate::{Point, Label, SiteID};
use serde::{Serialize, Deserialize};
#[cfg(feature="bevy")]
use bevy::prelude::{Component, Entity, Bundle};

/// Mark a point within the map of a level to serve as a ground truth relative
/// to other levels.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature="bevy", derive(Bundle))]
pub struct Fiducial<T: SiteID> {
    /// The anchor that represents the position of this fiducial.
    pub anchor: Point<T>,
    /// Label of this fiducial. This label must be unique within the level that
    /// the fiducial is being defined on. To be used for aligning, there must
    /// be a fiducial with the same label on one or more other levels. A value
    /// of None means it will not effect alignment.
    pub label: Label,
    #[serde(skip)]
    pub marker: FiducialMarker,
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct FiducialMarker;

#[cfg(feature="bevy")]
impl Fiducial<Entity> {
    pub fn to_u32(&self, anchor: u32) -> Fiducial<u32> {
        Fiducial{label: self.label.clone(), anchor: anchor.into(), marker: Default::default()}
    }
}

#[cfg(feature="bevy")]
impl Fiducial<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Fiducial<Entity> {
        Fiducial{
            anchor: self.anchor.to_ecs(id_to_entity),
            label: self.label.clone(),
            marker: Default::default(),
        }
    }
}

impl<T: SiteID> From<Point<T>> for Fiducial<T> {
    fn from(anchor: Point<T>) -> Self {
        Self{
            anchor,
            label: Default::default(),
            marker: Default::default(),
        }
    }
}
