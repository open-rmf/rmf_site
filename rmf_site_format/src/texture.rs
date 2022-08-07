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

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum TextureSource {
    Filename(String),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Texture {
    pub source: TextureSource,
    #[serde(skip_serializing_if="Option::is_none")]
    pub alpha: Option<f32>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub rotation: Option<Angle>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub scale: Option<f32>,
    #[serde(skip_serializing_if="Option::is_none")]
    pub offset: Option<(f32, f32)>,
}
