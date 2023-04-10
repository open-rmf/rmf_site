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

use bevy::prelude::*;
use crate::CurrentWorkspace;
use crate::interaction::{InteractionAssets, Selectable};
use crate::site::SiteAssets;
use rmf_site_format::WorkcellProperties;

/// Used as an event to command that a new workcell should be made the current one
#[derive(Clone, Copy, Debug)]
pub struct ChangeCurrentWorkcell {
    /// What should the current workcell root be
    pub root: Entity,
}

pub fn change_workcell(
    mut current_workspace: ResMut<CurrentWorkspace>,
    mut change_current_workcell: EventReader<ChangeCurrentWorkcell>,
    open_workcells: Query<Entity, With<WorkcellProperties>>,
) {
    if let Some(cmd) = change_current_workcell.iter().last() {
        if open_workcells.get(cmd.root).is_err() {
            println!(
                "Requested workspace change to an entity that is not an open workcell: {:?}",
                cmd.root
            );
            return;
        }

        current_workspace.root = Some(cmd.root);
        current_workspace.display = true;
    }
}

pub fn add_workcell_visualization(
    mut commands: Commands,
    new_workcells: Query<Entity, Added<WorkcellProperties>>,
    site_assets: Res<SiteAssets>,
    interaction_assets: Res<InteractionAssets>,
) {
    for e in new_workcells.iter() {
        let body_mesh = site_assets.site_anchor_mesh.clone();
        let mut entity_commands = commands.entity(e);
        entity_commands.add_children(|parent| {
            let mut body = parent.spawn(PbrBundle {
                mesh: body_mesh,
                material: site_assets.passive_anchor_material.clone(),
                ..default()
            });
            body.insert(Selectable::new(e));
        });
        interaction_assets.make_orientation_cue_meshes(&mut commands, e, 1.0);
    }
}
