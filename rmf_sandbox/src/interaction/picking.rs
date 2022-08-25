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
use bevy_mod_picking::{PickableMesh, PickingCamera};
use std::collections::HashSet;

/// A resource to track what kind of picking blockers are currently active
pub struct PickingBlockers {
    /// An InteractionMask entity is being hovered over
    pub masked: bool,
}

impl PickingBlockers {
    pub fn blocking(&self) -> bool {
        self.masked
    }
}

impl Default for PickingBlockers {
    fn default() -> Self {
        PickingBlockers { masked: false }
    }
}

/// Keep track of what entity is currently picked by the cursor
#[derive(Debug, Clone, Copy, Default)]
pub struct Picked(pub Option<Entity>);

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
    camera_controls: Query<&CameraControls, Changed<CameraControls>>,
    picking_cams: Query<Entity, With<PickingCamera>>,
) {
    for controls in &camera_controls {
        let active_camera = controls.active_camera();
        if picking_cams
            .get_single()
            .ok()
            .filter(|current| *current == active_camera)
            .is_none()
        {
            for cam in picking_cams.iter() {
                commands.entity(cam).remove_bundle::<PickingCameraBundle>();
            }

            commands
                .entity(controls.active_camera())
                .insert_bundle(PickingCameraBundle::default());
        }
    }
}

pub fn update_picked(
    blockers: Option<Res<PickingBlockers>>,
    pick_source_query: Query<&PickingCamera>,
    mut picked: ResMut<Picked>,
    mut change_pick: EventWriter<ChangePick>,
) {
    if let Some(blockers) = blockers {
        if blockers.blocking() {
            // If picking is masked, then nothing should be picked
            if picked.0.is_some() {
                change_pick.send(ChangePick{from: picked.0, to: None});
                picked.as_mut().0 = None;
            }
        }
    }

    let mut current_picked = None;
    for pick_source in &pick_source_query {
        if let Some(picks) = pick_source.intersect_list() {
            for (topmost_entity, _) in picks.iter() {
                current_picked = Some(*topmost_entity);
                break;
            }
        }
    }

    if picked.0 != current_picked {
        change_pick.send(ChangePick{from: picked.0, to: current_picked});
        picked.as_mut().0 = current_picked;
    }
}
