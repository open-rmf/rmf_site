/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use crate::{Edge, RefTrait};
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Constraint<T: RefTrait> {
    pub edge: Edge<T>,
    /// Marker that tells bevy the entity is a Constraint-type
    #[serde(skip)]
    pub marker: ConstraintMarker,
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct ConstraintMarker;

impl<T: RefTrait> Constraint<T> {
    pub fn convert<U: RefTrait>(
        &self,
        id_map: &HashMap<T, U>,
    ) -> Result<Constraint<U>, T> {
        Ok(Constraint {
            edge: self.edge.convert(id_map)?,
            marker: Default::default(),
        })
    }
}

impl<T: RefTrait> From<Edge<T>> for Constraint<T> {
    fn from(edge: Edge<T>) -> Self {
        Constraint {
            edge,
            marker: Default::default(),
        }
    }
}
