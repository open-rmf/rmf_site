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
use std::collections::HashSet;
use bevy::prelude::*;

#[derive(Bundle, Debug)]
pub struct AnchorBundle {
    anchor: Anchor,
    dependents: AnchorDependents,
    visibility: Visibility,
    computed: ComputedVisibility,
    transform: Transform,
    global_transform: GlobalTransform,
    category: Category,
}

impl AnchorBundle {
    pub fn new(anchor: (f32, f32)) -> Self {
        let transform = Transform::from_translation([anchor.0, anchor.1, 0.].into());
        Self{
            transform,
            global_transform: transform.into(),
            anchor: Default::default(),
            dependents: Default::default(),
            visibility: Default::default(),
            computed: Default::default(),
            category: Category("Anchor".to_string()),
        }
    }

    pub fn visible(self, is_visible: bool) -> Self {
        Self{
            visibility: Visibility{is_visible},
            ..self
        }
    }

    /// When the parent's GlobalTransform is not an identity matrix, this can
    /// be used to make sure the initial GlobalTransform of the anchor entity
    /// is immediately correct. Bevy's builtin transform propagation system will
    /// make sure it is correct after one update cycle, but that could mean that
    /// the anchor and its dependents have the wrong values until that cycle is
    /// finished.
    pub fn parent_transform(self, parent_tf: &GlobalTransform) -> Self {
        Self{
            global_transform: parent_tf.mul_transform(self.transform),
            ..self
        }
    }
}

#[derive(Component, Clone, Copy, Debug, Default)]
pub struct Anchor;

#[derive(Component, Debug, Default, Clone)]
pub struct AnchorDependents {
    pub dependents: HashSet<Entity>,
}

/// The PreviewAnchor component is held by exactly one Anchor entity that will
/// follow the cursor when the interaction mode is to add a new Anchor.
#[derive(Component)]
pub struct PreviewAnchor {
    /// If the preview anchor will be replacing an existing anchor, then this
    /// field keeps track of which anchor is being replaced. This information
    /// is helpful for sending dependents back to their original anchor if the
    /// user cancels the add-anchor interaction mode.
    replacing: Option<Entity>,
}
