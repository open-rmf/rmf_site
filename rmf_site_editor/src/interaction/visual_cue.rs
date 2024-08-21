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

use crate::interaction::{VISUAL_CUE_RENDER_LAYER, XRAY_RENDER_LAYER};
use bevy::{prelude::*, render::view::visibility::RenderLayers};
use bitfield::bitfield;
use smallvec::SmallVec;

bitfield! {
    #[derive(Copy, Clone)]
    pub struct VisibilityDependents(u8);
    impl Debug;
    // Always visible
    #[inline]
    pub always, set_always: 0;
    // Visible because the entity is selected
    #[inline]
    pub selected, set_selected: 1;
    // Visibile because the entity is hovered
    #[inline]
    pub hovered, set_hovered: 2;
    // Visible because it is supporting an entity that wants it to always be visible
    #[inline]
    pub support_always, set_support_always: 3;
    // Visible because the entity is supporting the selected entity
    #[inline]
    pub support_selected, set_support_selected: 4;
    // Visible because the entity is supporting the hovered entity
    #[inline]
    pub support_hovered, set_support_hovered: 5;
    // Visible because the entity is within proximity of the cursor
    #[inline]
    pub proximity, set_proximity: 6;
    // Visible because the entity is currently unassigned
    #[inline]
    pub unassigned, set_unassigned: 7;
}

impl VisibilityDependents {
    pub fn new_always() -> VisibilityDependents {
        VisibilityDependents(1)
    }

    pub fn new_none() -> VisibilityDependents {
        VisibilityDependents(0)
    }

    pub fn none(&self) -> bool {
        self.0 == 0
    }

    pub fn any(&self) -> bool {
        !self.none()
    }

    pub fn union(mut self, other: Self) -> Self {
        self.0 = self.0 | other.0;
        self
    }
}

/// A component to tag entities that are only meant to be visual cues and
/// should be excluded from visualization or analysis of physical objects.
#[derive(Component, Debug, Clone, Copy)]
pub struct VisualCue {
    /// Allow this visual cue to be outlined when it is interacted with
    pub allow_outline: bool,
    /// Whether to show this visual cue in the regular visual cue layer
    pub regular: VisibilityDependents,
    /// If this is not empty then the visual cue will be rendered over anything
    /// that would normally obstruct its view
    pub xray: VisibilityDependents,
}

/// Copied from VisualCue or inherited from parents
#[derive(Component, Debug, Clone, Copy, Deref, DerefMut)]
pub struct ComputedVisualCue(pub VisualCue);

/// Apply this to structs which should never be viewed with the xray camera
/// (i.e. they should never be rendered over objects that are in front of them)
#[derive(Component, Debug, Clone, Copy)]
pub struct DisableXray;

impl VisualCue {
    pub fn outline() -> VisualCue {
        VisualCue {
            allow_outline: true,
            regular: VisibilityDependents::new_always(),
            xray: VisibilityDependents::new_none(),
        }
    }

    pub fn no_outline() -> VisualCue {
        VisualCue {
            allow_outline: false,
            regular: VisibilityDependents::new_always(),
            xray: VisibilityDependents::new_none(),
        }
    }

    pub fn irregular(mut self) -> VisualCue {
        self.regular.set_always(false);
        self
    }

    pub fn always_xray(mut self) -> VisualCue {
        self.xray.set_always(true);
        self
    }

    pub fn layers(&self) -> RenderLayers {
        let mut layers = RenderLayers::none();
        if self.regular.any() {
            layers = layers.with(VISUAL_CUE_RENDER_LAYER);
        }
        if self.xray.any() {
            layers = layers.with(XRAY_RENDER_LAYER);
        }
        layers
    }

    pub fn with_constraints(mut self, constraints: VisualCueConstraints) -> Self {
        if constraints.disable_xray {
            // Shift all xray visibility over to the regular visibility
            self.regular = self.regular.union(self.xray);
            // Then eliminate all xray visibility
            self.xray = VisibilityDependents::new_none();
        }
        self
    }
}

pub struct VisualCueConstraints {
    pub disable_xray: bool,
}

impl VisualCueConstraints {
    pub fn new(disable_xray: bool) -> Self {
        Self { disable_xray }
    }
}

/// This system propagates visual cue tags and the correct RenderLayer to all
/// entities that are meant to be visual cues. This system makes two assumptions:
/// 1. Any entity that is a VisualCue will be a VisualCue forever
/// 2. Any descendents of a VisualCue should also be VisualCues.
/// We will need to change the implementation of this system if we ever want to
/// loosen either of those assumptions.
pub fn propagate_visual_cues(
    mut commands: Commands,
    changed_cues: Query<(Entity, &VisualCue), Or<(Changed<VisualCue>, Changed<Children>)>>,
    children: Query<&Children>,
    existing_cues: Query<(), With<VisualCue>>,
    disabled_xray: Query<(), With<DisableXray>>,
) {
    for (e, root_cue) in &changed_cues {
        let mut queue = SmallVec::<[(Entity, VisualCue); 5]>::new();
        queue.push((e, root_cue.clone()));
        while let Some((top, top_cue)) = queue.pop() {
            commands
                .entity(top)
                .insert(top_cue.layers())
                .insert(ComputedVisualCue(top_cue));

            if let Ok(children) = children.get(top) {
                for child in children {
                    if existing_cues.contains(*child) {
                        continue;
                    }

                    let disable_xray = disabled_xray.get(*child).is_ok();
                    let constraints = VisualCueConstraints::new(disable_xray);
                    queue.push((*child, top_cue.clone().with_constraints(constraints)));
                }
            }
        }
    }
}
