/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use crate::{interaction::*, site::*};
use bevy_rich_text3d::*;

pub fn update_door_interactive_cues(
    changed_doors: Query<
        (&Hovered, &Selected, &DoorSegments),
        Or<(Changed<Hovered>, Changed<Selected>)>,
    >,
    mut styles: Query<&mut Text3dStyling>,
    mut materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
    assets: Res<SiteAssets>,
) {
    for (hovered, selected, segments) in &changed_doors {
        let text_color = if hovered.cue() && selected.cue() {
            HOVER_SELECT_COLOR
        } else if hovered.cue() {
            HOVER_COLOR
        } else if selected.cue() {
            SELECT_COLOR
        } else {
            Color::BLACK
        };

        for text in segments.name_displays() {
            if let Ok(mut style) = styles.get_mut(text) {
                style.color = text_color.into();
            }
        }

        let cue_material = if hovered.cue() {
            assets.door_cue_highlighted_material.clone()
        } else {
            assets.door_cue_material.clone()
        };

        if let Ok(mut material) = materials.get_mut(segments.cue) {
            **material = cue_material;
        }
    }
}
