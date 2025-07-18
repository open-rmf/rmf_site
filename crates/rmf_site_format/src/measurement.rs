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
use bevy::prelude::{Bundle, Component, Deref, DerefMut, Entity};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Measurement {
    pub anchors: Edge,
    #[serde(skip_serializing_if = "is_default")]
    pub distance: Distance,
    #[serde(skip)]
    pub marker: MeasurementMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct Distance(pub Option<f32>);

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct MeasurementMarker;

impl Default for Distance {
    fn default() -> Self {
        Self(None)
    }
}

impl Measurement {
    pub fn convert(&self, id_map: &HashMap<SiteID, Entity>) -> Result<Measurement, SiteID> {
        Ok(Measurement {
            anchors: self.anchors.convert(id_map)?,
            distance: self.distance,
            marker: Default::default(),
        })
    }
}

impl From<Edge> for Measurement {
    fn from(anchors: Edge) -> Self {
        Self {
            anchors,
            distance: Default::default(),
            marker: Default::default(),
        }
    }
}
