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

use crate::interaction::*;
use bevy::{ecs::system::SystemParam, prelude::*};

/// Used as a resource to indicate what type of interaction we are currently
/// expecting from the user.
#[derive(Clone, Resource)]
pub enum InteractionMode {
    /// The user may hover/select any item in the scene. This is the default
    /// interaction mode.
    Inspect,
    /// The user must select an
    SelectAnchor(SelectAnchor),
    /// 3D version of SelectAnchor
    // TODO(anyone) rename above SelectAnchor2D and this SelectAnchor
    SelectAnchor3D(SelectAnchor3D),
}

impl Default for InteractionMode {
    fn default() -> Self {
        Self::Inspect
    }
}

impl InteractionMode {
    pub fn is_selecting(&self) -> bool {
        match self {
            Self::Inspect => true,
            Self::SelectAnchor(_) => true,
            Self::SelectAnchor3D(_) => true,
            // _ => false,
        }
    }

    pub fn is_selecting_anchor(&self) -> bool {
        match self {
            Self::SelectAnchor(_) => true,
            _ => false,
        }
    }

    pub fn is_inspecting(&self) -> bool {
        matches!(self, InteractionMode::Inspect)
    }

    fn backout<'w, 's>(&mut self, params: &mut BackoutParams<'w, 's>) {
        let change_mode = match self {
            Self::Inspect => {
                params.select.send(Select(None));
                // No change of mode is needed
                None
            }
            Self::SelectAnchor(select_anchor) => {
                Some(select_anchor.backout(&mut params.select_anchor))
            }
            Self::SelectAnchor3D(select_anchor) => {
                Some(select_anchor.backout(&mut params.select_anchor))
            }
        };

        if let Some(change_mode) = change_mode {
            *self = change_mode;
        }
    }

    fn cancel<'w, 's>(&mut self, params: &mut BackoutParams<'w, 's>) {
        while !matches!(self, InteractionMode::Inspect) {
            self.backout(params);
        }
    }
}

#[derive(SystemParam)]
pub struct BackoutParams<'w, 's> {
    select: EventWriter<'w, Select>,
    select_anchor: SelectAnchorPlacementParams<'w, 's>,
}

/// We use an event to change the InteractionMode for these reasons:
/// 1. By having a single system where the mode resource is changed, we can
///    ensure that the InteractionMode is consistent through the whole update
///    cycle. If the mode were changed part way through a cycle, then the
///    behavior of various systems might not be coherent with each other.
/// 2. Certain mode transitions require cleanup before being finalized. We
///    cannot expect users to know this or handle the cleanup correctly.
///
/// User-defined systems should never
/// use ResMut<InteractionMode>. Instead they should always use
/// EventWriter<ChangeMode>.
//
// TODO(MXG): We could enforce this by letting InteractionMode be public but
// wrapping it in a newtype to store it in the resource. The inner type would
// be kept private (with read-only access) so only this module can modify it.
#[derive(Event)]
pub enum ChangeMode {
    /// Change the mode to the specified value
    To(InteractionMode),
    /// Backout of the current mode. This is the behavior usually associated
    /// with a user pressing Esc.
    Backout,
}

pub fn update_interaction_mode(
    mut mode: ResMut<InteractionMode>,
    mut change_mode: EventReader<ChangeMode>,
    mut backout: BackoutParams,
) {
    for request in change_mode.read() {
        match request {
            ChangeMode::To(new_mode) => {
                mode.cancel(&mut backout);
                *mode = new_mode.clone();
            }
            ChangeMode::Backout => {
                mode.backout(&mut backout);
            }
        }
    }
}
