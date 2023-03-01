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

use crate::site::*;
use bevy::prelude::*;
use smallvec::SmallVec;

/// What is the display layer for this entity. This will determine a material
/// offset bias to prevent z-fighting between flat entities that are being
/// rendered at the same height.
#[derive(Debug, Clone, Copy, Component, PartialEq, Eq, PartialOrd, Ord)]
pub struct DisplayLayer(pub i32);

pub fn update_layers_from_rankings(
    mut commands: Commands,
    level_rankings: Query<(
        &RecencyRanking<FloorMarker>,
        &RecencyRanking<DrawingMarker>,
    ),
    (
        Or<(
            Changed<RecencyRanking<FloorMarker>>,
            Changed<RecencyRanking<DrawingMarker>>,
        )>,
    )>,
    site_rankings: Query<
        &RecencyRanking<NavGraphMarker>,
        Changed<RecencyRanking<NavGraphMarker>>,
    >,
    mut layers: Query<&mut DisplayLayer>,
) {
    for (floor_ranking, drawing_ranking) in &level_rankings {
        dbg!();
        // We use negative values for floors and drawings to keep their values
        // independent from the nav graph rankings which use positive values.
        // Floors and drawings should always be displayed below nav graph
        // elements.
        let lowest_depth = (drawing_ranking.len() + floor_ranking.len()) as i32;
        for (pos, entity) in drawing_ranking.iter().chain(floor_ranking.iter()).enumerate() {
            let current_layer = DisplayLayer(pos as i32 - lowest_depth);
            if let Ok(mut previous_layer) = layers.get_mut(*entity) {
                if *previous_layer != current_layer {
                    *previous_layer = current_layer;
                }
            } else {
                commands.entity(*entity).insert(current_layer);
            }
        }
    }

    for graph_ranking in &site_rankings {
        // dbg!(graph_ranking);
        for (pos, entity) in graph_ranking.iter().enumerate() {
            // dbg!(entity);
            let current_layer = DisplayLayer(pos as i32 + 1);
            if let Ok(mut previous_layer) = layers.get_mut(*entity) {
                if *previous_layer != current_layer {
                    // dbg!(entity);
                    *previous_layer = current_layer;
                }
            } else {
                // dbg!(entity);
                commands.entity(*entity).insert(current_layer);
            }
        }
    }
}
