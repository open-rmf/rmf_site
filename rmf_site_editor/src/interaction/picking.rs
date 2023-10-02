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

use crate::{interaction::*, site::Anchor, CurrentWorkspace};
use bevy::prelude::*;
use bevy_mod_picking::{PickableMesh, PickingCamera, PickingCameraBundle};

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
pub struct Picked(pub Option<Entity>);

#[derive(Debug, Clone, Copy, Default)]
pub struct ChangePick {
    pub from: Option<Entity>,
    pub to: Option<Entity>,
}

#[derive(Bundle, Default)]
pub struct PickableBundle {
    pub pickable_mesh: PickableMesh,
}

pub fn update_picking_cam(
    mut commands: Commands,
    camera_controls: Res<CameraControls>,
    picking_cams: Query<Entity, With<PickingCamera>>,
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
                commands.entity(cam).remove::<PickingCameraBundle>();
            }

            commands
                .entity(camera_controls.active_camera())
                .insert(PickingCameraBundle::default());
        }
    }
}

fn pick_topmost(
    picks: impl Iterator<Item = Entity>,
    selectable: &Query<&Selectable>,
    anchors: &Query<&Parent, (With<Anchor>, Without<Preview>)>,
    mode: &Res<InteractionMode>,
    current_site: Entity,
) -> Option<Entity> {
    for topmost_entity in picks {
        match &**mode {
            InteractionMode::SelectAnchor(request) => {
                if let Ok(sel) = selectable.get(topmost_entity) {
                    if sel.is_selectable {
                        if let Ok(parent) = anchors.get(sel.element) {
                            if request.site_scope() && parent.get() != current_site {
                                continue;
                            }
                        } else {
                            continue;
                        }
                    } else {
                        continue;
                    }
                } else {
                    continue;
                }
            }
            _ => {
                // Do nothing
            }
        }
        return Some(topmost_entity);
    }

    return None;
}

pub fn update_picked(
    mode: Res<InteractionMode>,
    selectable: Query<&Selectable>,
    anchors: Query<&Parent, (With<Anchor>, Without<Preview>)>,
    blockers: Option<Res<PickingBlockers>>,
    pick_source_query: Query<&PickingCamera>,
    visual_cues: Query<&ComputedVisualCue>,
    mut picked: ResMut<Picked>,
    mut change_pick: EventWriter<ChangePick>,
    current_workspace: Res<CurrentWorkspace>,
    mut gpu_pick_event: EventReader<GPUPickItem>,
) {
    if let Some(blockers) = blockers {
        if blockers.blocking() {
            // If picking is masked, then nothing should be picked
            if picked.0.is_some() {
                change_pick.send(ChangePick {
                    from: picked.0,
                    to: None,
                });
                picked.as_mut().0 = None;
            }

            return;
        }
    }

    let current_site = match current_workspace.root {
        Some(current_site) => current_site,
        None => return,
    };

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
                &anchors,
                &mode,
                current_site,
            ) {
                break 'current_picked Some(topmost);
            }

            // Now look at all possible pickables
            if let Some(topmost) = pick_topmost(
                picks.iter().map(|(e, _)| *e),
                &selectable,
                &anchors,
                &mode,
                current_site,
            ) {
                break 'current_picked Some(topmost);
            }
        }
        // Use GPU picking if nothing is picked via normal picking
        if !gpu_pick_event.is_empty() {
            if let Some(item) = gpu_pick_event.iter().next() {
                break 'current_picked Some(item.0);
            }
        } 

        None
    };

    if picked.0 != current_picked {
        change_pick.send(ChangePick {
            from: picked.0,
            to: current_picked,
        });
        picked.as_mut().0 = current_picked;
    }
}
