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
use bevy::prelude::*;
use bevy_mod_raycast::{deferred::RaycastSource, immediate::RaycastVisibility};

/// A resource to track what kind of picking blockers are currently active
#[derive(Resource)]
pub struct PickingBlockers {
    /// An InteractionMask entity is being hovered over
    pub masked: bool,
    /// The UI is being hovered over
    pub ui: bool,
}

impl PickingBlockers {
    pub fn blocking(&self) -> bool {
        self.masked || self.ui
    }
}

impl Default for PickingBlockers {
    fn default() -> Self {
        PickingBlockers {
            masked: false,
            ui: false,
        }
    }
}

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

pub fn update_picking_cam(
    mut commands: Commands,
    camera_controls: Res<CameraControls>,
    picking_cams: Query<Entity, With<RaycastSource<SiteRaycastSet>>>,
) {
    if camera_controls.is_changed() {
        let active_camera = camera_controls.active_camera();
        if picking_cams
            .get_single()
            .ok()
            .filter(|current| *current == active_camera)
            .is_none()
        {
            for cam in picking_cams.iter() {
                commands
                    .entity(cam)
                    .remove::<RaycastSource<SiteRaycastSet>>();
            }

            commands.entity(camera_controls.active_camera()).insert(
                RaycastSource::<SiteRaycastSet>::new_cursor()
                    .with_early_exit(false)
                    .with_visibility(RaycastVisibility::MustBeVisible),
            );
        }
    }
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
    blockers: Option<Res<PickingBlockers>>,
    pick_source_query: Query<&RaycastSource<SiteRaycastSet>>,
    visual_cues: Query<&ComputedVisualCue>,
    mut picked: ResMut<Picked>,
    mut change_pick: EventWriter<ChangePick>,
) {
    if let Some(blockers) = blockers {
        if blockers.blocking() {
            // If picking is masked, then nothing should be picked
            if picked.current.is_some() {
                change_pick.send(ChangePick {
                    from: picked.current,
                    to: None,
                });
                picked.current = None;
            }

            return;
        }
    }

    let current_picked = 'current_picked: {
        for pick_source in &pick_source_query {
            let picks = pick_source.intersections();
            // First only look at the visual cues that are being xrayed
            if let Some(topmost) = pick_topmost(
                picks
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
            if let Some(topmost) = pick_topmost(picks.iter().map(|(e, _)| *e), &selectable) {
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
        change_pick.send(ChangePick {
            from: picked.current,
            to: current_picked,
        });
        picked.current = current_picked;
    }
}
