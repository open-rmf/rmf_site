/*
 * Copyright (C) 2023 Intrinsic LLC
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
use bevy::prelude::{Bundle, Component, Entity};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct GeoReference<T: RefTrait> {
    /// The anchor that represents the position of this fiducial.
    pub anchor: T,
    pub latitude: f32,
    pub longitude: f32
}

#[cfg(feature = "bevy")]
impl GeoReference<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> GeoReference<Entity> {
        GeoReference {
            anchor: *id_to_entity.get(&self.anchor).unwrap(),
            latitude: self.latitude,
            longitude: self.longitude
        }
    }
}

#[cfg(feature = "bevy")]
impl GeoReference<Entity> {
    pub fn to_u32(&self, anchor: u32) -> GeoReference<u32> {
        GeoReference {
            anchor: anchor.into(),
            latitude: self.latitude,
            longitude: self.longitude
        }
    }
}