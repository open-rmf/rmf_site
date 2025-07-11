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

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct PhysicalCamera {
    pub name: NameInSite,
    pub pose: Pose,
    pub properties: PhysicalCameraProperties,
    #[serde(skip)]
    pub previewable: PreviewableMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct PhysicalCameraProperties {
    pub width: u32,
    pub height: u32,
    pub horizontal_fov: Angle,
    pub frame_rate: f32,
}

impl Default for PhysicalCameraProperties {
    fn default() -> Self {
        PhysicalCameraProperties {
            width: 1280,
            height: 720,
            horizontal_fov: Angle::Deg(90.0),
            frame_rate: 30.0,
        }
    }
}
