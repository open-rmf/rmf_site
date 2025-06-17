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

use std::collections::HashMap;

use crate::interaction::*;
use bevy::{picking::pointer::PointerInteraction, prelude::*};
use bytemuck::TransparentWrapper;
use rmf_site_camera::{resources::BlockStatus, TypeInfo};


/// The UI is being hovered over
#[derive(Resource, Default, TransparentWrapper)]
#[repr(transparent)]
pub struct UiHovered(pub bool);

/// An InteractionMask entity is being hovered over
#[derive(Resource, Default, TransparentWrapper)]
#[repr(transparent)]
pub struct IteractionMaskHovered(pub bool);

#[derive(Reflect, Resource, Default, TransparentWrapper)]
#[reflect(Resource)]
#[repr(transparent)]
pub struct PickingBlockers(pub HashMap<TypeInfo, bool>);

pub type PickBlockerRegistration<T> = BlockerRegistration<T, PickingBlockers>;

pub type PickBlockStatus = BlockStatus<PickingBlockers>;

/// Keep track of what entity is currently picked by the cursor
#[derive(Debug, Clone, Copy, Default, Resource)]
pub struct Picked {
    /// This is the currently picked entity (if anything)
    pub current: Option<Entity>,
    /// This indicates that a workflow wants the current pick to be refreshed
    /// even if it hasn't changed. If this is true, we will send a ChangePick
    /// event on the next cycle.
    pub refresh: bool,
}

#[derive(Debug, Clone, Copy, Default, Event)]
pub struct ChangePick {
    pub from: Option<Entity>,
    pub to: Option<Entity>,
}

fn pick_topmost(
    picks: impl Iterator<Item = Entity>,
    selectable: &Query<&Selectable>,
) -> Option<Entity> {
    for topmost_entity in picks {
        if let Ok(sel) = selectable.get(topmost_entity) {
            if !sel.is_selectable {
                continue;
            }
        } else {
            continue;
        }

        return Some(topmost_entity);
    }

    return None;
}

// TODO(@mxgrey): Consider making this a service similar to hover_service and select_service
pub fn update_picked(
    selectable: Query<&Selectable>,
    block_status: Res<PickBlockStatus>,
    pointers: Query<&PointerInteraction>,
    visual_cues: Query<&ComputedVisualCue>,
    mut picked: ResMut<Picked>,
    mut change_pick: EventWriter<ChangePick>,
) {
    //if let Some(blockers) = pick_status {
    if block_status.blocked() {
        // If picking is masked, then nothing should be picked
        if picked.current.is_some() {
            change_pick.write(ChangePick {
                from: picked.current,
                to: None,
            });
            picked.current = None;
        }

        return;
    }
    //}

    let current_picked = 'current_picked: {
        for interactions in &pointers {
            // First only look at the visual cues that are being xrayed
            if let Some(topmost) = pick_topmost(
                interactions
                    .iter()
                    .filter(|(e, _)| {
                        visual_cues
                            .get(*e)
                            .ok()
                            .filter(|cue| cue.xray.any())
                            .is_some()
                    })
                    .map(|(e, _)| *e),
                &selectable,
            ) {
                break 'current_picked Some(topmost);
            }

            // Now look at all possible pickables
            if let Some(topmost) = pick_topmost(interactions.iter().map(|(e, _)| *e), &selectable) {
                break 'current_picked Some(topmost);
            }
        }

        None
    };

    let refresh = picked.refresh;
    if refresh {
        picked.refresh = false;
    }

    if picked.current != current_picked || refresh {
        change_pick.write(ChangePick {
            from: picked.current,
            to: current_picked,
        });
        picked.current = current_picked;
    }
}
