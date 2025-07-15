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

#[derive(Serialize, Deserialize, Default, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct Texture {
    pub source: AssetSource,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub alpha: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rotation: Option<Angle>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub width: Option<f32>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub height: Option<f32>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
#[cfg_attr(feature = "bevy", derive(Reflect))]
pub struct TextureGroup {
    pub name: NameInSite,
    // The flatten attribute currently does not work correctly for the .ron
    // format, so we cannot use it for now.
    // #[serde(flatten)]
    pub texture: Texture,
    #[serde(skip)]
    pub group: Group,
}
