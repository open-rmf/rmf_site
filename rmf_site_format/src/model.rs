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
use bevy::prelude::{Component, Bundle};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature="bevy", derive(Bundle))]
pub struct Model {
    /// Name of the model instance
    pub name: NameInSite,
    /// What kind of model is this (i.e. its SDF Model name). If None, nothing
    /// will be loaded for it.
    pub kind: Label,
    /// Pose of the model relative to the level it is on.
    pub pose: Pose,
    #[serde(skip_serializing_if="is_default")]
    /// Whether this model should be able to move in simulation
    pub is_static: IsStatic,
    /// Only relevant for bevy
    #[serde(skip)]
    pub marker: ModelMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature="bevy", derive(Component))]
pub struct ModelMarker;
