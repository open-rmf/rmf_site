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
use bevy::prelude::{Bundle, Component};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Wall<T: RefTrait> {
    pub anchors: Edge<T>,
    #[serde(skip_serializing_if = "is_default")]
    pub texture: Affiliation<T>,
    #[serde(skip)]
    pub marker: WallMarker,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct WallMarker;

impl<T: RefTrait> Wall<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Wall<U>, T> {
        Ok(Wall {
            anchors: self.anchors.convert(id_map)?,
            texture: self.texture.convert(id_map)?,
            marker: Default::default(),
        })
    }
}

impl<T: RefTrait> From<Edge<T>> for Wall<T> {
    fn from(anchors: Edge<T>) -> Self {
        Self {
            anchors,
            texture: Affiliation(None),
            marker: Default::default(),
        }
    }
}
