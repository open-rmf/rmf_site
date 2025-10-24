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
pub struct EdgeVisualCue {
    /// If the edge is using support from some anchors, the entities of those
    /// anchors will be saved here.
    supporters: Option<Edge<Entity>>,
}

pub fn add_edge_visual_cues(
    mut commands: Commands,
    new_edges: Query<(Entity, &Edge<Entity>), Without<EdgeVisualCue>>,
) {
    for (e, edge) in &new_edges {
        commands.entity(e).insert(EdgeVisualCue {
            supporters: Some(*edge),
        });
    }
}

pub fn update_edge_visual_cues(
    mut edges: Query<
        (
            Entity,
            &Hovered,
            &Selected,
            &Edge<Entity>,
            &mut EdgeVisualCue,
        ),
        (
            Without<AnchorVisualization>,
            Without<Point<Entity>>,
            Without<Path<Entity>>,
            Or<(Changed<Hovered>, Changed<Selected>, Changed<Edge<Entity>>)>,
        ),
    >,
    mut anchors: Query<(&mut Hovered, &mut Selected), With<AnchorVisualization>>,
) {
    for (e, hovered, selected, edge, mut cue) in &mut edges {
        let [a0, a1] = edge.array();
        if let Some(old) = cue.supporters {
            // If we have supporters that are out of date, clear them out.
            // This can happen if a user changes the start or end vertices
            // of the lane.
            if old.array() != [a0, a1] {
                for v in old.array() {
                    if let Ok((mut hover, mut selected)) = anchors.get_mut(v) {
                        hover.support_hovering.remove(&e);
                        selected.support_selected.remove(&e);
                    }
                }
            }
        }

        if hovered.cue() || selected.cue() {
            cue.supporters = Some(*edge);
        } else {
            cue.supporters = None;
        }

        if let Ok(
            [
                (mut hovered_a0, mut selected_a0),
                (mut hover_a1, mut selected_a1),
            ],
        ) = anchors.get_many_mut([a0, a1])
        {
            if hovered.cue() {
                hovered_a0.support_hovering.insert(e);
                hover_a1.support_hovering.insert(e);
            } else {
                hovered_a0.support_hovering.remove(&e);
                hover_a1.support_hovering.remove(&e);
            }

            if selected.cue() {
                selected_a0.support_selected.insert(e);
                selected_a1.support_selected.insert(e);
            } else {
                selected_a0.support_selected.remove(&e);
                selected_a1.support_selected.remove(&e);
            }
        }
    }
}
