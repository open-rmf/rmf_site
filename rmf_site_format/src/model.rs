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
use bevy::prelude::{Bundle, Component, Reflect, ReflectComponent};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Model {
    /// Name of the model instance
    pub name: NameInSite,
    /// Where the model should be loaded from
    pub source: AssetSource,
    /// Pose of the model relative to the level it is on.
    pub pose: Pose,
    #[serde(default, skip_serializing_if = "is_default")]
    /// Whether this model should be able to move in simulation
    pub is_static: IsStatic,
    /// Scale to be applied to the model
    #[serde(default, skip_serializing_if = "is_default")]
    pub scale: Scale,
    /// Only relevant for bevy
    #[serde(skip)]
    pub marker: ModelMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct ModelMarker;

impl Default for Model {
    fn default() -> Self {
        Self {
            name: NameInSite("<Unnamed>".to_string()),
            source: AssetSource::default(),
            pose: Pose::default(),
            is_static: IsStatic(false),
            scale: Scale::default(),
            marker: ModelMarker,
        }
    }
}

///
///
///

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct ModelDescription {
    pub name: NameInSite,
    pub source: AssetSource,
    #[serde(default, skip_serializing_if = "is_default")]
    #[serde(skip)]
    pub group: Group,
    #[serde(skip)]
    pub marker: ModelMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct ModelInstance<T: RefTrait> {
    pub name: NameInSite,
    #[serde(skip)]
    pub source: AssetSource,
    pub pose: Pose,
    pub parent: SiteParentID,
    pub description: Affiliation<T>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub is_static: IsStatic,
    #[serde(default, skip_serializing_if = "is_default")]
    pub scale: Scale,
    #[serde(skip)]
    pub marker: ModelMarker,
}

impl<T: RefTrait> Default for ModelInstance<T> {
    fn default() -> Self {
        Self {
            name: NameInSite("<Unnamed>".to_string()),
            source: AssetSource::default(),
            pose: Pose::default(),
            parent: SiteParentID(0),
            description: Affiliation::default(),
            is_static: IsStatic(false),
            scale: Scale::default(),
            marker: ModelMarker,
        }
    }
}

impl<T: RefTrait> ModelInstance<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<ModelInstance<U>, T> {
        Ok(ModelInstance {
            name: self.name.clone(),
            source: self.source.clone(),
            pose: self.pose.clone(),
            parent: self.parent.clone(),
            description: self.description.convert(id_map)?,
            is_static: self.is_static.clone(),
            scale: self.scale.clone(),
            marker: Default::default(),
        })
    }
}
