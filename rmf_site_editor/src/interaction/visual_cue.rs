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

use bevy::{
    prelude::*,
    render::view::visibility::RenderLayers,
};
use smallvec::SmallVec;
use crate::interaction::VISUAL_CUE_RENDER_LAYER;

/// A unit component to tag entities that are only meant to be visual cues and
/// should be excluded from visualization or analysis of physical objects.
#[derive(Component, Debug)]
pub struct VisualCue;

/// This system propagates visual cue tags and the correct RenderLayer to all
/// entities that are meant to be visual cues. This system makes two assumptions:
/// 1. Any entity that is a VisualCue will be a VisualCue forever
/// 2. Any descendents of a VisualCue should also be VisualCues.
/// We will need to change the implementation of this system if we ever want to
/// loosen either of those assumptions.
pub fn propagate_visual_cues(
    mut commands: Commands,
    new_cues: Query<Entity, Or<(Added<VisualCue>, (With<VisualCue>, Changed<Children>))>>,
    children: Query<&Children>,
) {
    for e in &new_cues {
        let mut queue = SmallVec::<[Entity; 5]>::new();
        queue.push(e);
        while let Some(top) = queue.pop() {
            commands.entity(top)
                .insert(VisualCue)
                .insert(RenderLayers::layer(VISUAL_CUE_RENDER_LAYER));
            if let Ok(children) = children.get(top) {
                for child in children {
                    queue.push(*child);
                }
            }
        }
    }
}
