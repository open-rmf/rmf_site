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

use crate::{Affiliation, Group, NameInSite, Point, RefTrait};
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Entity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Mark a point within a drawing or level to serve as a ground truth relative
/// to other drawings and levels.
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Fiducial<T: RefTrait> {
    /// The anchor that represents the position of this fiducial.
    pub anchor: Point<T>,
    /// Affiliation of this fiducial. This affiliation must be unique within the
    /// parent level or parent drawing of the fiducial.
    pub affiliation: Affiliation<T>,
    #[serde(skip)]
    pub marker: FiducialMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct FiducialGroup {
    /// Name of this group
    pub name: NameInSite,
    #[serde(skip)]
    pub group: Group,
    #[serde(skip)]
    pub marker: FiducialMarker,
}

impl FiducialGroup {
    pub fn new(name: NameInSite) -> Self {
        Self {
            name,
            group: Default::default(),
            marker: Default::default(),
        }
    }
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct FiducialMarker;

impl<T: RefTrait> Fiducial<T> {
    pub fn convert<U: RefTrait>(
        &self,
        id_map: &HashMap<T, U>,
    ) -> Result<Fiducial<U>, T> {
        Ok(Fiducial {
            anchor: self.anchor.convert(id_map)?,
            affiliation: self.affiliation.convert(id_map)?,
            marker: Default::default(),
        })
    }
}

impl<T: RefTrait> From<Point<T>> for Fiducial<T> {
    fn from(anchor: Point<T>) -> Self {
        Self {
            anchor,
            affiliation: Default::default(),
            marker: Default::default(),
        }
    }
}
