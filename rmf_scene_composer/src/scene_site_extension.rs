/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use crate::SceneSubscription;

use librmf_site_editor::site::{
    ExtensionHooks, AssignSiteID, LevelElevation, Pose, SavingArgs, SiteID,
};

use bevy::{
    prelude::*,
    ecs::system::SystemState,
};

use serde::{Serialize, Deserialize};

use std::collections::BTreeMap;

#[derive(Serialize, Deserialize)]
struct SceneInfo {
    subscription: String,
    pose: Pose,
    level: u32,
}

use thiserror::Error as ThisError;

fn save_scenes(
    SavingArgs { site }: SavingArgs,
    world: &mut World,
) -> Result<serde_json::Value, SceneSavingError> {

    let mut data = BTreeMap::<u32, SceneInfo>::new();

    let mut state: SystemState<(
        Query<&SiteID, With<LevelElevation>>,
        Query<&Children>,
        Query<(&SceneSubscription, &Pose)>,
        AssignSiteID
    )> = SystemState::new(world);
    let (levels, children, scenes, mut assign_site_id) = state.get_mut(world);

    let mut next_site_id = assign_site_id
        .assign_for(site)
        .ok_or(SceneSavingError::InvalidSite)?;

    for level in children.get(site).map_err(|_| SceneSavingError::InvalidSite)? {
        let Ok(level_id) = levels.get(*level) else {
            continue;
        };

        let Ok(level_children) = children.get(*level) else {
            continue;
        };

        for level_child in level_children {
            if let Ok((subscription, pose)) = scenes.get(*level_child) {
                let id = next_site_id.assign_to(*level_child);
                data.insert(id, SceneInfo {
                    subscription: subscription.topic_name().to_owned(),
                    pose: *pose,
                    level: level_id.0,
                });
            }
        }
    }

    state.apply(world);

    serde_json::to_value(data)
        .map_err(SceneSavingError::SerializationFailure)
}

#[derive(ThisError, Debug)]
enum SceneSavingError {
    #[error("The entity we were told to save is not a valid site")]
    InvalidSite,
    #[error("Failed to serialize: {0}")]
    SerializationFailure(serde_json::Error),
}
