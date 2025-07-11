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

#[derive(Component, Default)]
pub struct PathVisualCue {
    supporters: Option<Path>,
}

pub fn add_path_visual_cues(
    mut commands: Commands,
    new_paths: Query<(Entity, &Path), Without<PointVisualCue>>,
) {
    for (e, path) in &new_paths {
        commands.entity(e).insert(PathVisualCue {
            supporters: Some(path.clone()),
        });
    }
}

pub fn update_path_visual_cues(
    mut paths: Query<
        (Entity, &Hovered, &Selected, &Path, &mut PathVisualCue),
        (
            Without<AnchorVisualization>,
            Without<Edge>,
            Without<Point>,
            Or<(Changed<Hovered>, Changed<Selected>, Changed<Path>)>,
        ),
    >,
    mut anchors: Query<(&mut Hovered, &mut Selected), With<AnchorVisualization>>,
) {
    for (p, hovered, selected, path, mut cue) in &mut paths {
        if let Some(old) = &cue.supporters {
            // If we have supporters that are out of date, clear them out.
            // This can happen if a user changes a reference anchor for the
            // path.
            if *old != *path {
                for anchor in &old.0 {
                    if let Ok((mut hover, mut selected)) = anchors.get_mut(*anchor) {
                        hover.support_hovering.remove(&p);
                        selected.support_selected.remove(&p);
                    }
                }
            }
        }

        if hovered.cue() || selected.cue() {
            cue.supporters = Some(path.clone());
        } else {
            cue.supporters = None;
        }

        for anchor in &path.0 {
            if let Ok((mut anchor_hovered, mut anchor_selected)) = anchors.get_mut(*anchor) {
                if hovered.cue() {
                    anchor_hovered.support_hovering.insert(p);
                } else {
                    anchor_hovered.support_hovering.remove(&p);
                }

                if selected.cue() {
                    anchor_selected.support_selected.insert(p);
                } else {
                    anchor_selected.support_selected.remove(&p);
                }
            }
        }
    }
}
