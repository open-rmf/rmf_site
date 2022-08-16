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

use std::collections::HashSet;
use bevy::prelude::*;

#[derive(Component)]
pub struct Anchor(pub f32, pub f32);

impl Anchor {
    pub fn vec(&self) -> Vec2 {
        Vec2::new(self.0, self.1)
    }

    pub fn x(&self) -> f32 {
        self.0
    }

    pub fn y(&self) -> f32 {
        self.1
    }

    pub fn transform(&self) -> Transform {
        Transform::from_xyz(self.0, self.1, 0.0)
    }
}

impl From<Anchor> for (f32, f32) {
    fn from(anchor: Anchor) -> Self {
        (anchor.0, anchor.1)
    }
}

#[derive(Component)]
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

pub fn update_changed_anchor_visuals(
    mut anchors: Query<(&Anchor, &mut Transform), Changed<Anchor>>,
) {
    for (anchor, mut tf) in &mut anchors {
        tf.translation = anchor.transform();
    }
}
