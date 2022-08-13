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
    site::*,
};
use rmf_site_format::Lane;
use bevy::prelude::*;


#[derive(Component, Default)]
pub struct LaneVisualCue {
    /// If the lane is using support from some vertices, the entities of those
    /// vertices will be noted here
    supporters: Option<(Entity, Entity)>,
}

pub fn update_lane_visual_cues(
    mut lanes: Query<
        (
            Entity,
            &Hovering,
            &Selected,
            &Lane<Entity>,
            &LaneSegments,
            &mut LaneVisualCue,
            &mut Transform,
        ),
        (
            Without<AnchorVisualCue>,
            Or<(
                Changed<Hovering>,
                Changed<Selected>,
                Changed<Lane<Entity>>,
            )>,
        ),
    >,
    mut vertices: Query<(&mut Hovering, &mut Selected), With<AnchorVisualCue>>,
    mut materials: Query<&mut Handle<StandardMaterial>>,
    mut visibility: Query<&mut Visibility>,
    site_assets: Res<SiteAssets>,
    cursor: Res<Cursor>,
) {
    for (l, hovering, selected, lane, pieces, mut cue, mut tf) in &mut lanes {
        let (v0, v1) = lane.anchors;
        if let Some((old_v0, old_v1)) = cue.supporters {
            // If we have supporters that are out of date, clear them out.
            // This can happen if a user changes the start or end vertices
            // of the lane.
            if (old_v0, old_v1) != (v0, v1) {
                for v in [old_v0, old_v1] {
                    if let Some((mut hover, mut selected)) = vertices.get_mut(v).ok() {
                        hover.support_hovering.remove(&l);
                        selected.support_selected.remove(&l);
                    }
                }
            }
        }

        if hovering.cue() || selected.cue() {
            cue.supporters = Some((v0, v1));
        } else {
            cue.supporters = None;
        }

        if let Some([(mut hover_v0, mut selected_v0), (mut hover_v1, mut selected_v1)]) =
            vertices.get_many_mut([v0, v1]).ok()
        {
            if hovering.cue() {
                hover_v0.support_hovering.insert(l);
                hover_v1.support_hovering.insert(l);
            } else {
                hover_v0.support_hovering.remove(&l);
                hover_v1.support_hovering.remove(&l);
            }

            if selected.cue() {
                selected_v0.support_selected.insert(l);
                selected_v1.support_selected.insert(l);
            } else {
                selected_v0.support_selected.remove(&l);
                selected_v1.support_selected.remove(&l);
            }
        }

        if hovering.is_hovering {
            set_visibility(cursor.frame, &mut visibility, false);
        }

        let (m, h) = if hovering.cue() && selected.cue() {
            (&site_assets.hover_select_material, HOVERED_LANE_HEIGHT)
        } else if hovering.cue() {
            (&site_assets.hover_material, HOVERED_LANE_HEIGHT)
        } else if selected.cue() {
            (&site_assets.select_material, SELECTED_LANE_HEIGHT)
        } else {
            (&site_assets.passive_lane_material, PASSIVE_LANE_HEIGHT)
        };

        for e in pieces.segments {
            set_material(e, m, &mut materials);
        }

        tf.translation.z = h;
    }
}
