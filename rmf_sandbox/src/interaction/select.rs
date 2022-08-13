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
use bevy_mod_picking::PickableBundle;

#[derive(Component)]
pub struct Selectable {
    /// Toggle whether this entity is selectable
    pub is_selectable: bool,
    /// What element of the site is being selected when this entity is clicked
    pub element: Entity,
}

impl Selectable {
    fn new(element: Entity) -> Self {
        Selectable{is_selectable: true, element}
    }
}

#[derive(Component, Debug, PartialEq, Eq)]
pub struct Hovering {
    /// The cursor is hovering on this object specifically
    pub is_hovering: bool,
    /// The cursor is hovering on a different object which wants this entity
    /// to be highlighted.
    pub support_hovering: HashSet<Entity>,
}

impl Hovering {
    pub fn cue(&self) -> bool {
        self.is_hovering || !self.support_hovering.is_empty()
    }
}

impl Default for Hovering {
    fn default() -> Self {
        Self {
            is_hovering: false,
            support_hovering: Default::default(),
        }
    }
}

#[derive(Component, Debug, PartialEq, Eq)]
pub struct Selected {
    /// This object has been selected
    pub is_selected: bool,
    /// Another object is selected but wants this entity to be highlighted
    pub support_selected: HashSet<Entity>,
}

impl Selected {
    pub fn cue(&self) -> bool {
        self.is_selected || !self.support_selected.is_empty()
    }
}

impl Default for Selected {
    fn default() -> Self {
        Self {
            is_selected: false,
            support_selected: Default::default(),
        }
    }
}

pub fn make_selectable_entities_pickable(
    mut commands: Commands,
    new_selectables: Query<Entity, Added<Selectable>>,
) {
    for new_selectable in &new_selectables {
        commands.entity(new_selectable)
            .insert_bundle(PickableBundle::default());
    }
}
