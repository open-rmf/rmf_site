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

use crate::{
    site::*,
};
use bevy::prelude::*;

/// This component is applied to each site element that gets loaded in order to
/// remember what its original ID within the Site file was.
#[derive(Component, Clone, Copy, Debug)]
pub struct SiteID(pub u32);

pub struct LoadSite(rmf_site_format::Site);

pub fn load_site(
    new_site: Res<LoadSite>
) {
    // TODO(MXG) Construct entities from the new_site
}
