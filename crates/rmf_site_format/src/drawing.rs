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
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct PixelsPerMeter(pub f32);

impl Default for PixelsPerMeter {
    fn default() -> Self {
        PixelsPerMeter(100.0)
    }
}

#[derive(Default, Serialize, Deserialize, Debug, Clone)]
pub struct Drawing {
    // Even though round trip flattening is supposed to work after
    // https://github.com/ron-rs/ron/pull/455, it seems it currently fails
    // in ron, even forcing a dependency on that commit.
    // TODO(luca) investigate further, come up with a minimum example,
    // open an upstream issue and link it here for reference.
    // #[serde(flatten)]
    pub properties: DrawingProperties,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub anchors: BTreeMap<SiteID, Anchor>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fiducials: BTreeMap<SiteID, Fiducial>,
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub measurements: BTreeMap<SiteID, Measurement>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct DrawingProperties {
    pub name: NameInSite,
    pub source: AssetSource,
    pub pose: Pose,
    pub pixels_per_meter: PixelsPerMeter,
    #[serde(
        default = "PreferredSemiTransparency::for_drawing",
        skip_serializing_if = "PreferredSemiTransparency::is_default_for_drawing"
    )]
    pub preferred_semi_transparency: PreferredSemiTransparency,
}

impl Default for DrawingProperties {
    fn default() -> Self {
        Self {
            name: Default::default(),
            source: Default::default(),
            pose: Default::default(),
            pixels_per_meter: Default::default(),
            preferred_semi_transparency: PreferredSemiTransparency::for_drawing(),
        }
    }
}
