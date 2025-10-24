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

use crate::site::{DisplayColor, assets::old_default_material};
use bevy::prelude::*;

pub fn add_material_for_display_colors(
    mut commands: Commands,
    new_display: Query<(Entity, &DisplayColor), Added<DisplayColor>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, c) in &new_display {
        commands.entity(e).insert(MeshMaterial3d(
            materials.add(old_default_material(Color::srgb_from_array(c.0))),
        ));
    }
}

pub fn update_material_for_display_color(
    changed_color: Query<(&DisplayColor, &MeshMaterial3d<StandardMaterial>), Changed<DisplayColor>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (color, material) in &changed_color {
        if let Some(m) = materials.get_mut(material) {
            m.base_color = Color::srgb_from_array(color.0);
        }
    }
}
