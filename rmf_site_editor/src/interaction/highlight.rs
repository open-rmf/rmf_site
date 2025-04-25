/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use crate::{interaction::*, site::DrawingMarker};
use bevy::color::palettes::css as Colors;
use bevy::prelude::*;

#[derive(Component)]
pub struct Highlight {
    pub select: Color,
    pub hover: Color,
    pub hover_select: Color,
}

#[derive(Component)]
pub struct SuppressHighlight;

impl Highlight {
    pub fn for_drawing() -> Self {
        Self {
            select: Color::srgb(1., 0.7, 1.),
            hover: Color::srgb(0.7, 1., 1.),
            hover_select: Color::srgb(1.0, 0.5, 0.7),
        }
    }
}

pub fn add_highlight_visualization(
    mut commands: Commands,
    new_drawings: Query<Entity, Added<DrawingMarker>>,
) {
    for e in &new_drawings {
        commands.entity(e).insert(Highlight::for_drawing());
    }
}

pub fn update_highlight_visualization(
    highlightable: Query<
        (
            &Hovered,
            &Selected,
            &Handle<StandardMaterial>,
            &Highlight,
            Option<&SuppressHighlight>,
        ),
        Or<(
            Changed<Hovered>,
            Changed<Selected>,
            Changed<SuppressHighlight>,
            Changed<Handle<StandardMaterial>>,
        )>,
    >,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (hovered, selected, m, highlight, suppress) in &highlightable {
        if let Some(material) = materials.get_mut(m) {
            let mut color = if suppress.is_some() {
                Colors::WHITE.into()
            } else if hovered.cue() && selected.cue() {
                highlight.hover_select
            } else if hovered.cue() {
                highlight.hover
            } else if selected.cue() {
                highlight.select
            } else {
                Colors::WHITE.into()
            };
            color.set_alpha(material.base_color.alpha());

            material.base_color = color;
        }
    }
}
