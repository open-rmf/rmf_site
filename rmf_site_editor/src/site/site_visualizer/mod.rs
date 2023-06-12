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

use crate::interaction::{InteractionState, SetCategoryVisibility};
use crate::site::{CurrentLevel, LaneMarker, LevelProperties, SiteProperties};
use crate::{AppState, CurrentWorkspace};

use std::collections::HashSet;

#[derive(Default)]
pub struct SiteVisualizerPlugin;

fn show_all_levels(
    workspace: Res<CurrentWorkspace>,
    open_sites: Query<Entity, With<SiteProperties>>,
    children: Query<&Children>,
    mut levels: Query<(&mut Visibility, &mut Transform, &LevelProperties)>,
    mut lanes_visibility: EventWriter<SetCategoryVisibility<LaneMarker>>,
) {
    if let Some(children) = workspace
        .to_site(&open_sites)
        .and_then(|s| children.get(s).ok())
    {
        for child in children.iter() {
            if let Ok((mut vis, mut tf, properties)) = levels.get_mut(*child) {
                vis.is_visible = true;
                tf.translation.z = properties.elevation;
            }
        }
        lanes_visibility.send(false.into());
    }
}

fn hide_all_non_current_levels(
    workspace: Res<CurrentWorkspace>,
    open_sites: Query<Entity, With<SiteProperties>>,
    children: Query<&Children>,
    mut levels: Query<(&mut Visibility, &mut Transform), With<LevelProperties>>,
    current_level: Res<CurrentLevel>,
    mut lanes_visibility: EventWriter<SetCategoryVisibility<LaneMarker>>,
) {
    if let Some(children) = workspace
        .to_site(&open_sites)
        .and_then(|s| children.get(s).ok())
    {
        for child in children.iter() {
            if let Ok((mut vis, mut tf)) = levels.get_mut(*child) {
                vis.is_visible = Some(*child) == **current_level;
                tf.translation.z = 0.0;
            }
        }
        lanes_visibility.send(true.into());
    }
}

fn update_level_elevation(
    mut changed_levels: Query<(&mut Transform, &LevelProperties), Changed<LevelProperties>>,
) {
    for (mut tf, properties) in &mut changed_levels {
        tf.translation.z = properties.elevation;
    }
}

fn disable_interaction(mut interaction_state: ResMut<State<InteractionState>>) {
    println!("Entering site visualizer");
    interaction_state.set(InteractionState::Disable).ok();
}

fn enable_interaction(mut interaction_state: ResMut<State<InteractionState>>) {
    println!("Exiting site visualizer");
    interaction_state.set(InteractionState::Enable).ok();
}

impl Plugin for SiteVisualizerPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_enter(AppState::SiteVisualizer)
                .with_system(show_all_levels)
                .with_system(disable_interaction),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::SiteVisualizer)
                .with_system(hide_all_non_current_levels)
                .with_system(enable_interaction),
        )
        .add_system_set(
            SystemSet::on_update(AppState::SiteVisualizer).with_system(update_level_elevation),
        );
    }
}
