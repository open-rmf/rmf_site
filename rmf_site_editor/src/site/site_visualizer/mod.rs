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
use crate::site::{CurrentLevel, LaneMarker, LevelElevation, NameOfSite, SiteProperties};
use crate::{AppState, CurrentWorkspace};

#[derive(Default)]
pub struct SiteVisualizerPlugin;

fn show_all_levels(
    workspace: Res<CurrentWorkspace>,
    open_sites: Query<Entity, With<NameOfSite>>,
    children: Query<&Children>,
    mut levels: Query<(&mut Visibility, &mut Transform, &LevelElevation)>,
    mut lanes_visibility: EventWriter<SetCategoryVisibility<LaneMarker>>,
) {
    if let Some(children) = workspace
        .to_site(&open_sites)
        .and_then(|s| children.get(s).ok())
    {
        for child in children.iter() {
            if let Ok((mut vis, mut tf, elevation)) = levels.get_mut(*child) {
                *vis = Visibility::Inherited;
                tf.translation.z = elevation.0;
            }
        }
        lanes_visibility.send(false.into());
    }
}

fn hide_all_non_current_levels(
    workspace: Res<CurrentWorkspace>,
    open_sites: Query<Entity, With<NameOfSite>>,
    children: Query<&Children>,
    mut levels: Query<(&mut Visibility, &mut Transform), With<LevelElevation>>,
    current_level: Res<CurrentLevel>,
    mut lanes_visibility: EventWriter<SetCategoryVisibility<LaneMarker>>,
) {
    if let Some(children) = workspace
        .to_site(&open_sites)
        .and_then(|s| children.get(s).ok())
    {
        for child in children.iter() {
            if let Ok((mut vis, mut tf)) = levels.get_mut(*child) {
                *vis = if Some(*child) == **current_level {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
                tf.translation.z = 0.0;
            }
        }
        lanes_visibility.send(true.into());
    }
}

fn update_level_elevation(
    mut changed_levels: Query<(&mut Transform, &LevelElevation), Changed<LevelElevation>>,
) {
    for (mut tf, elevation) in &mut changed_levels {
        tf.translation.z = elevation.0;
    }
}

fn disable_interaction(mut interaction_state: ResMut<NextState<InteractionState>>) {
    info!("Entering site visualizer");
    interaction_state.set(InteractionState::Disable);
}

fn enable_interaction(mut interaction_state: ResMut<NextState<InteractionState>>) {
    info!("Exiting site visualizer");
    interaction_state.set(InteractionState::Enable);
}

impl Plugin for SiteVisualizerPlugin {
    fn build(&self, app: &mut App) {
        app.add_systems(
            OnEnter(AppState::SiteVisualizer),
            (show_all_levels, disable_interaction),
        )
        .add_systems(
            OnExit(AppState::SiteVisualizer),
            (hide_all_non_current_levels, enable_interaction),
        )
        .add_systems(
            Update,
            update_level_elevation.run_if(in_state(AppState::SiteVisualizer)),
        );
    }
}
