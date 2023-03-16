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

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct PixelsPerMeter(pub f32);

impl Default for PixelsPerMeter {
    fn default() -> Self {
        PixelsPerMeter(100.0)
    }
}

// TODO(MXG): Consider adding NameInSite field
#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Drawing {
    pub source: AssetSource,
    pub pose: Pose,
    pub pixels_per_meter: PixelsPerMeter,
    #[serde(skip)]
    pub marker: DrawingMarker,
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct DrawingMarker;
