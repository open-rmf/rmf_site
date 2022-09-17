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
use rmf_site_format::{Lane, Location, NavGraphProperties};

pub fn assign_orphans_to_nav_graph(
    mut commands: Commands,
    mut selected_nav_graph: ResMut<SelectedNavGraph>,
    new_elements: Query<Entity, Or<(Added<Lane<Entity>>, Added<Location<Entity>>)>>,
    parents: Query<&Parent, With<Lane>>,
) {
    if new_elements.is_empty() {
        return;
    }

    let mut get_selected_nav_graph = || -> Entity {
        if selected_nav_graph.0.is_none() {
            // Create a new nav graph since there isn't one selected right now
            let new_nav_graph = commands
                .spawn_bundle(SpatialBundle::default())
                .insert(NavGraphProperties{
                    name: "<Unnamed>".to_string()
                }).id();

            *selected_nav_graph = Some(new_nav_graph);
        }

        return selected_nav_graph.0;
    };

    for e in &new_elements {
        if !parents.contains(e) {
            // This new lane does not have a parent, so we should assign it to
            // the currently selected nav graph.
            commands.entity(get_selected_nav_graph()).add_child(e);
        }
    }
}
