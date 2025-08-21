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

use crate::{interaction::*, layers::ZLayer, site::*};
use bevy::prelude::*;
use rmf_site_format::{Edge, LaneMarker};

pub fn add_lane_visual_cues(
    mut commands: Commands,
    new_lane_segments: Query<(Entity, &LaneSegments), Added<LaneSegments>>,
) {
    for (e, segments) in &new_lane_segments {
        commands.entity(e).insert(VisualCue::no_outline());
        commands.entity(segments.mid).insert(Selectable::new(e));
    }
}

pub fn update_lane_visual_cues(
    mut lanes: Query<
        (&Hovered, &Selected, &LaneSegments, &mut Transform),
        (
            With<LaneMarker>,
            Without<AnchorVisualization>,
            Or<(Changed<Hovered>, Changed<Selected>, Changed<Edge<Entity>>)>,
        ),
    >,
    mut materials: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut visibility: Query<&mut Visibility>,
    site_assets: Res<SiteAssets>,
    cursor: Res<Cursor>,
) {
    for (hovered, selected, pieces, mut tf) in &mut lanes {
        if hovered.is_hovered {
            set_visibility(cursor.frame, &mut visibility, false);
        }

        let (m, h, v) = if hovered.cue() && selected.cue() {
            (
                &site_assets.hover_select_material,
                ZLayer::HoveredLane.to_z(),
                true,
            )
        } else if hovered.cue() {
            (
                &site_assets.hover_material,
                ZLayer::HoveredLane.to_z(),
                true,
            )
        } else if selected.cue() {
            (
                &site_assets.select_material,
                ZLayer::SelectedLane.to_z(),
                true,
            )
        } else {
            (&site_assets.unassigned_lane_material, 0.0, false)
        };

        for e in pieces.outlines {
            set_material(e, m, &mut materials);
            set_visibility(e, &mut visibility, v);
        }

        tf.translation.z = h;
    }
}
