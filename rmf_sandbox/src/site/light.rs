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

use bevy::prelude::*;
use rmf_site_format::Light;

// TODO(MXG): Give a way to interact with lights and change their properties
pub fn add_physical_lights(
    mut commands: Commands,
    physical_lights: Query<(Entity, &Light), Added<Light>>,
) {
    for (e, light) in &physical_lights {
        light.insert_into(&mut commands.entity(e));
    }
}
