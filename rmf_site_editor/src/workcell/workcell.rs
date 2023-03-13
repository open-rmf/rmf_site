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
use crate::interaction::{InteractionAssets, Selectable};
use crate::site::SiteAssets;
use rmf_site_format::WorkcellProperties;

pub fn add_workcell_visualization(
    mut commands: Commands,
    new_workcells: Query<Entity, Added<WorkcellProperties>>,
    site_assets: Res<SiteAssets>,
    interaction_assets: Res<InteractionAssets>,
) {
    for e in new_workcells.iter() {
        let body_mesh = site_assets.site_anchor_mesh.clone();
        let mut entity_commands = commands.entity(e);
        let body = entity_commands.add_children(|parent| {
            let mut body = parent.spawn(PbrBundle {
                mesh: body_mesh,
                material: site_assets.passive_anchor_material.clone(),
                ..default()
            });
            body.insert(Selectable::new(e));
            let body = body.id();
            // TODO(luca) make workcell not deletable

            body
        });
        interaction_assets.make_orientation_cue_meshes(&mut commands, e, 1.0);
    }
}
