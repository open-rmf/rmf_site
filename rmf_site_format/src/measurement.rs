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
pub struct Measurement<T: RefTrait> {
    pub anchors: Edge<T>,
    #[serde(skip_serializing_if = "is_default")]
    pub distance: Distance,
    #[serde(skip_serializing_if = "is_default")]
    pub label: Label,
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

#[cfg(feature = "bevy")]
impl Measurement<Entity> {
    pub fn to_u32(&self, anchors: Edge<u32>) -> Measurement<u32> {
        Measurement {
            anchors,
            distance: self.distance,
            label: self.label.clone(),
            marker: Default::default(),
        }
    }
}

impl<T: RefTrait> Measurement<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Measurement<U>, T> {
        Ok(Measurement {
            anchors: self.anchors.convert(id_map)?,
            distance: self.distance,
            label: self.label.clone(),
            marker: Default::default(),
        })
    }
}

impl<T: RefTrait> From<Edge<T>> for Measurement<T> {
    fn from(anchors: Edge<T>) -> Self {
        Self {
            anchors,
            distance: Default::default(),
            label: Default::default(),
            marker: Default::default(),
        }
    }
}
