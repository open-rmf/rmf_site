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

use crate::interaction::{VISUAL_CUE_RENDER_LAYER, VISUAL_CUE_XRAY_LAYER};
use bevy::{prelude::*, render::view::visibility::RenderLayers};
use smallvec::SmallVec;
use std::collections::HashSet;

/// A unit component to tag entities that are only meant to be visual cues and
/// should be excluded from visualization or analysis of physical objects.
#[derive(Component, Debug, Clone)]
pub struct VisualCue {
    /// Allow this visual cue to be outlined when it is interacted with
    pub allow_outline: bool,
    /// Whether to show this visual cue in the regular visual cue layer
    pub regular: bool,
    /// If this is not empty then the visual cue will be rendered over anything
    /// that would normally obstruct its view
    // TODO(MXG): Consider if HashSet is overkill and whether we could use a
    // data structure that does not need heap allocation.
    pub xray_dependents: HashSet<u32>,
    /// Did this entity's visual cue setting get inherited from another
    pub inherited: bool,
}

impl VisualCue {
    pub fn outline() -> VisualCue {
        VisualCue {
            allow_outline: true,
            regular: true,
            xray_dependents: HashSet::new(),
            inherited: false
        }
    }

    pub fn no_outline() -> VisualCue {
        VisualCue {
            allow_outline: false,
            regular: true,
            xray_dependents: HashSet::new(),
            inherited: false
        }
    }

    pub fn irregular(mut self) -> VisualCue {
        self.regular = false;
        self
    }

    pub fn inherit(other: Self) -> VisualCue {
        VisualCue {
            inherited: true,
            ..other
        }
    }

    pub fn xray_active(&self) -> bool {
        return !self.xray_dependents.is_empty();
    }

    pub fn layers(&self) -> RenderLayers {
        let mut layers = RenderLayers::none();
        if self.regular {
            layers = layers.with(VISUAL_CUE_RENDER_LAYER);
        }
        if self.xray_active() {
            layers = layers.with(VISUAL_CUE_XRAY_LAYER);
        }
        layers
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
    new_cues: Query<(Entity, &VisualCue), Or<(Changed<VisualCue>, Changed<Children>)>>,
    children: Query<&Children>,
    existing_cues: Query<&VisualCue>,
) {
    for (e, root_cue) in &new_cues {
        let mut queue = SmallVec::<[(Entity, VisualCue); 5]>::new();
        queue.push((e, root_cue.clone()));
        while let Some((top, top_cue)) = queue.pop() {
            commands
                .entity(top)
                .insert(top_cue.layers())
                .insert(top_cue.clone());

            if let Ok(children) = children.get(top) {
                for child in children {
                    let child_cue = if let Ok(existing_cue) = existing_cues.get(*child) {
                        if existing_cue.inherited {
                            VisualCue::inherit(top_cue.clone())
                        } else {
                            existing_cue.clone()
                        }
                    } else {
                        VisualCue::inherit(top_cue.clone())
                    };
                    queue.push((*child, child_cue));
                }
            }
        }
    }
}
