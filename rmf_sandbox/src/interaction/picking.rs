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

use crate::{
    interaction::*,
};
use bevy::prelude::*;
use std::collections::HashSet;

/// Marker component for entities that block interaction with any other entities
/// while the cursor is hovering over them.
#[derive(Component)]
pub struct InteractionMask;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum Blockers {
    /// A UI element or some other InteractionMask is in the way of the cursor
    Masked,
    /// An item is being dragged
    Dragging,
    /// An item is being placed
    Placing,
}

/// A resource to track what kind of interaction blockers are currently active
pub struct CurrentBlockers(HashSet<Blockers>);

pub fn update_masks(
    mut blockers: ResMut<CurrentBlockers>,
) {

}
