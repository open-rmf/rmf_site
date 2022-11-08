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

use crate::site::*;
use bevy::prelude::*;

pub fn update_level_visibility(
    mut levels: Query<(Entity, &mut Visibility), With<LevelProperties>>,
    current_level: Res<CurrentLevel>,
) {
    if current_level.is_changed() {
        for (e, mut visibility) in &mut levels {
            visibility.is_visible = Some(e) == **current_level;
        }
    }
}
