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

use crate::{interaction::*, site::Anchor};
use bevy::prelude::*;
use std::collections::HashSet;

/// This component is put on entities with meshes to mark them as items that can
/// be interacted with to
#[derive(Component, Clone, Copy, Debug)]
pub struct Selectable {
    /// Toggle whether this entity is selectable
    pub is_selectable: bool,
    /// What element of the site is being selected when this entity is clicked
    pub element: Entity,
}

impl Selectable {
    pub fn new(element: Entity) -> Self {
        Selectable {
            is_selectable: true,
            element,
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

/// Component to track whether an element should be viewed in the Hovered state
/// for the selection tool.
#[derive(Component, Debug, PartialEq, Eq)]
pub struct Hovered {
    /// The cursor is hovering on this object specifically
    pub is_hovered: bool,
    /// The cursor is hovering on a different object which wants this entity
    /// to be highlighted.
    pub support_hovering: HashSet<Entity>,
}

impl Hovered {
    pub fn cue(&self) -> bool {
        self.is_hovered || !self.support_hovering.is_empty()
    }
}

impl Default for Hovered {
    fn default() -> Self {
        Self {
            is_hovered: false,
            support_hovering: Default::default(),
        }
    }
}

/// Used as a resource to keep track of which entity is currently selected.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Resource)]
pub struct Selection(pub Option<Entity>);

/// Used as a resource to keep track of which entity is currently hovered.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Resource)]
pub struct Hovering(pub Option<Entity>);

/// Used as an event to command a change in the selected entity.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut)]
pub struct Select(pub Option<Entity>);

/// Used as an event to command a change in the hovered entity.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut)]
pub struct Hover(pub Option<Entity>);

/// A resource to track what kind of blockers are preventing the selection
/// behavior from being active
#[derive(Resource)]
pub struct SelectionBlockers {
    /// An entity is being dragged
    pub dragging: bool,
    /// An entity is being placed
    pub placing: bool,
}

impl SelectionBlockers {
    pub fn blocking(&self) -> bool {
        self.dragging || self.placing
    }
}

impl Default for SelectionBlockers {
    fn default() -> Self {
        SelectionBlockers {
            dragging: false,
            placing: false,
        }
    }
}

pub fn make_selectable_entities_pickable(
    mut commands: Commands,
    new_selectables: Query<(Entity, &Selectable), Added<Selectable>>,
    targets: Query<(Option<&Hovered>, Option<&Selected>)>,
) {
    for (entity, selectable) in &new_selectables {
        commands.entity(entity).insert(PickableBundle::default());

        if let Ok((hovered, selected)) = targets.get(selectable.element) {
            if hovered.is_none() {
                commands
                    .entity(selectable.element)
                    .insert(Hovered::default());
            }

            if selected.is_none() {
                commands
                    .entity(selectable.element)
                    .insert(Selected::default());
            }
        }
    }
}

pub fn handle_selection_picking(
    blockers: Option<Res<SelectionBlockers>>,
    mode: Res<InteractionMode>,
    selectables: Query<&Selectable>,
    anchors: Query<(), With<Anchor<Entity>>>,
    mut picks: EventReader<ChangePick>,
    mut hover: EventWriter<Hover>,
) {
    if let Some(blockers) = blockers {
        if blockers.blocking() {
            hover.send(Hover(None));
            return;
        }
    }

    if !mode.is_selecting() {
        hover.send(Hover(None));
        return;
    }

    for pick in picks.iter() {
        hover.send(Hover(
            pick.to
                .and_then(|change_pick_to| {
                    selectables
                        .get(change_pick_to)
                        .ok()
                        .map(|selectable| selectable.element)
                })
                .and_then(|change_pick_to| {
                    if let InteractionMode::SelectAnchor(_) = *mode {
                        if anchors.contains(change_pick_to) {
                            Some(change_pick_to)
                        } else {
                            None
                        }
                    } else {
                        Some(change_pick_to)
                    }
                }),
        ));
    }
}

pub fn maintain_hovered_entities(
    mut hovered: Query<&mut Hovered>,
    mut hovering: ResMut<Hovering>,
    mut hover: EventReader<Hover>,
    mouse_button_input: Res<Input<MouseButton>>,
    touch_input: Res<Touches>,
    mut select: EventWriter<Select>,
    mode: Res<InteractionMode>,
    blockers: Option<Res<PickingBlockers>>,
) {
    if let Some(new_hovered) = hover.iter().last() {
        if hovering.0 != new_hovered.0 {
            if let Some(previous_hovered) = hovering.0 {
                if let Ok(mut hovering) = hovered.get_mut(previous_hovered) {
                    hovering.is_hovered = false;
                }
            }

            if let Some(new_hovered) = new_hovered.0 {
                if let Ok(mut hovering) = hovered.get_mut(new_hovered) {
                    hovering.is_hovered = true;
                }
            }

            hovering.0 = new_hovered.0;
        }
    }

    let clicked = mouse_button_input.just_pressed(MouseButton::Left)
        || touch_input.iter_just_pressed().next().is_some();
    let blocked = blockers.filter(|x| x.blocking()).is_some();

    if clicked && !blocked {
        if let Some(current_hovered) = hovering.0 {
            // TODO(luca) refactor to remove this hack
            // Skip if we are in SelectAnchor3D mode
            if let InteractionMode::SelectAnchor3D(mode) = &*mode {
                if mode.begin_creating() {
                    return;
                }
            }
            select.send(Select(Some(current_hovered)));
        }
    }
}

pub fn maintain_selected_entities(
    mode: Res<InteractionMode>,
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
    mut select: EventReader<Select>,
) {
    if !mode.is_inspecting() {
        // We only maintain the "selected" entity when we are in Inspect mode.
        // Other "selecting" modes, like SelectAnchor, take in the selection as
        // an event and do not change the current selection that is being
        // inspected.
        return;
    }

    if let Some(new_selection) = select.iter().last() {
        if selection.0 != new_selection.0 {
            if let Some(previous_selection) = selection.0 {
                if let Ok(mut selected) = selected.get_mut(previous_selection) {
                    selected.is_selected = false;
                }
            }

            if let Some(new_selection) = new_selection.0 {
                if let Ok(mut selected) = selected.get_mut(new_selection) {
                    selected.is_selected = true;
                }
            }

            selection.0 = new_selection.0;
        }
    }
}
