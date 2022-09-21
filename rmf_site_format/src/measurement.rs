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
use bevy::prelude::{Component, Entity, Bundle, Deref, DerefMut};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature="bevy", derive(Bundle))]
pub struct Measurement<T: SiteID> {
    pub anchors: Edge<T>,
    #[serde(skip_serializing_if="is_default")]
    pub distance: Distance,
    #[serde(skip_serializing_if="is_default")]
    pub label: Label,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature="bevy", derive(Component, Deref, DerefMut))]
pub struct Distance(pub Option<f32>);

impl Default for Distance {
    fn default() -> Self {
        Self(None)
    }
}

#[cfg(feature="bevy")]
impl Measurement<Entity> {
    pub fn to_u32(&self, anchors: Edge<u32>) -> Measurement<u32> {
        Measurement{
            anchors,
            distance: self.distance,
            label: self.label.clone()
        }
    }
}

#[cfg(feature="bevy")]
impl Measurement<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Measurement<Entity> {
        Measurement{
            anchors: self.anchors.to_ecs(id_to_entity),
            distance: self.distance,
            label: self.label.clone(),
        }
    }
}

impl<T: SiteID> From<Edge<T>> for Measurement<T> {
    fn from(anchors: Edge<T>) -> Self {
        Self{
            anchors,
            distance: Default::default(),
            label: Default::default(),
        }
    }
}
