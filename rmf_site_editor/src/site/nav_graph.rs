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

use bevy::prelude::*;
use rmf_site_format::{LaneMarker, LocationTags, NavGraphProperties};

/// Used as a resource to identify which Nav Graph is the currently selected one
#[derive(Debug, Default, Clone, Copy)]
pub struct SelectedNavGraph(pub Option<Entity>);

pub fn assign_orphans_to_nav_graph(
    mut commands: Commands,
    mut selected_nav_graph: ResMut<SelectedNavGraph>,
    new_elements: Query<
        Entity,
        (
            Or<(Added<LaneMarker>, Added<LocationTags>)>,
            Without<Parent>,
        ),
    >,
) {
    if new_elements.is_empty() {
        return;
    }

    let mut get_selected_nav_graph = || -> Entity {
        if let Some(nav_graph) = selected_nav_graph.0 {
            nav_graph
        } else {
            // Create a new nav graph since there isn't one selected right now
            let new_nav_graph = commands
                .spawn_bundle(SpatialBundle::default())
                .insert(NavGraphProperties {
                    name: "<Unnamed>".to_string(),
                })
                .id();

            selected_nav_graph.0 = Some(new_nav_graph);
            new_nav_graph
        }
    };

    for e in &new_elements {
        // This new lane or location does not have a parent, so we should assign
        // it to the currently selected nav graph.
    }
}
