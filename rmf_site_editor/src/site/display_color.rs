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

use crate::site::DisplayColor;
use bevy::prelude::*;

pub fn add_material_for_display_colors(
    mut commands: Commands,
    new_nav_graph: Query<(Entity, &DisplayColor), Added<DisplayColor>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (e, c) in &new_nav_graph {
        commands
            .entity(e)
            .insert(materials.add(Color::rgba(c.0[0], c.0[1], c.0[2], c.0[3]).into()));
    }
}

pub fn update_material_for_display_color(
    changed_color: Query<(&DisplayColor, &Handle<StandardMaterial>), Changed<DisplayColor>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (color, material) in &changed_color {
        if let Some(m) = materials.get_mut(material) {
            m.base_color = color.0.into();
        }
    }
}
