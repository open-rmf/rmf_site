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

#[derive(Component, Default)]
pub struct PointVisualCue {
    /// If the point is using support from an anchor, the entity of that
    /// anchor will be saved here.
    supporter: Option<Point>,
}

pub fn add_point_visual_cues(
    mut commands: Commands,
    new_points: Query<(Entity, &Point), Without<PointVisualCue>>,
) {
    for (e, point) in &new_points {
        commands.entity(e).insert(PointVisualCue {
            supporter: Some(*point),
        });
    }
}

pub fn update_point_visual_cues(
    mut points: Query<
        (
            Entity,
            &Hovered,
            &Selected,
            &Point,
            &mut PointVisualCue,
        ),
        (
            Without<AnchorVisualization>,
            Without<Edge>,
            Without<Path>,
            Or<(Changed<Hovered>, Changed<Selected>, Changed<Point>)>,
        ),
    >,
    mut anchors: Query<(&mut Hovered, &mut Selected), With<AnchorVisualization>>,
) {
    for (p, hovered, selected, point, mut cue) in &mut points {
        let anchor = point.0;
        if let Some(old) = cue.supporter {
            // If we have a supporter that is out of date, clear it out.
            // This can happen if a user changes the reference anchor for the
            // point.
            if old.0 != anchor {
                if let Ok((mut hover, mut selected)) = anchors.get_mut(*old.0) {
                    hover.support_hovering.remove(&p);
                    selected.support_selected.remove(&p);
                }
            }
        }

        if hovered.cue() || selected.cue() {
            cue.supporter = Some(*point);
        } else {
            cue.supporter = None;
        }

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
