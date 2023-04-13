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

use crate::{interaction::*, site::*};
use bevy::prelude::*;

#[derive(Component, Clone, Copy)]
pub struct CellSelectionVisual(Entity);

pub fn add_passage_cell_visual_cues(
    mut commands: Commands,
    new_passage_cells: Query<Entity, Added<CellTag>>,
) {
    for e in &new_passage_cells {
        commands
            .entity(e)
            .insert(VisualCue::no_outline())
            .insert(Selectable::new(e));
    }
}

pub fn update_passage_cell_visual_cues(
    mut commands: Commands,
    changed_cells: Query<
        (Entity, &CellTag, &Hovered, &Selected, Option<&CellSelectionVisual>),
        Or<(Changed<Hovered>, Changed<Selected>)>
    >,
    passages: Query<&PassageSkeleton>,
    mut highlight: Query<&mut Handle<StandardMaterial>>,
    site_assets: Res<SiteAssets>,
) {
    for (e, tag, hovered, selected, visual) in &changed_cells {
        let Ok(skeleton) = passages.get(tag.for_passage) else { continue };
        let material = if hovered.cue() {
            skeleton.compass.hovered.clone()
        } else if selected.cue() {
            skeleton.compass.selected.clone()
        } else {
            if let Some(visual) = visual {
                commands.entity(visual.0).despawn_recursive();
                commands.entity(e).remove::<CellSelectionVisual>();
            }
            continue;
        };

        if let Some(visual) = visual {
            if let Ok(mut mat) = highlight.get_mut(visual.0) {
                *mat = material;
                continue;
            }

            // Something weird happened with the highlighter so let's despawn it
            commands.entity(visual.0).despawn_recursive();
            commands.entity(e).remove::<CellSelectionVisual>();
        }

        let v = commands.spawn(PbrBundle {
            transform: Transform::from_xyz(0.0, 0.0, -0.0002),
            material,
            mesh: site_assets.unit_square_flat_mesh.clone(),
            ..default()
        }).id();

        commands
            .entity(e)
            .add_child(v)
            .insert(CellSelectionVisual(v));
    }
}
