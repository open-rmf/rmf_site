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

use crate::{SceneSubscription, SceneSubscriber};

use librmf_site_editor::site::{
    ExtensionHooks, ExtensionSettings, AssignSiteID, LevelElevation, Pose, SavingArgs, LoadingArgs, SiteID,
    SetSiteExtensionHook,
};

use bevy::prelude::*;

use serde::{Serialize, Deserialize};

use std::collections::{BTreeMap, HashMap};

#[derive(Default)]
pub struct SceneSiteExtensionPlugin {}

impl Plugin for SceneSiteExtensionPlugin {
    fn build(&self, app: &mut App) {
        app.world.set_site_extension_hook(
            "gz-scene",
            ExtensionSettings::default(),
            save_scenes,
            load_scenes,
        );
    }
}

#[derive(Serialize, Deserialize)]
struct SceneInfo {
    topic_name: String,
    pose: Pose,
    level: u32,
}

use thiserror::Error as ThisError;

fn save_scenes(
    In(SavingArgs { site }): In<SavingArgs>,
    levels: Query<&SiteID, With<LevelElevation>>,
    children: Query<&Children>,
    scenes: Query<(&SceneSubscription, &Pose)>,
    mut assign_site_id: AssignSiteID
) -> Result<serde_json::Value, SceneSavingError> {

    let mut data = BTreeMap::<u32, SceneInfo>::new();
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
                    topic_name: subscription.topic_name().to_owned(),
                    pose: *pose,
                    level: level_id.0,
                });
            }
        }
    }

    Ok(serde_json::to_value(data)?)
}

fn load_scenes(
    In(LoadingArgs { site, data }): In<LoadingArgs>,
    mut subscriber: SceneSubscriber,
    children: Query<&Children>,
    levels: Query<&SiteID, With<LevelElevation>>,
    mut commands: Commands,
) -> Result<(), SceneLoadingError> {
    let data: BTreeMap<u32, SceneInfo> = serde_json::from_value(data)?;
    let mut level_map = HashMap::new();
    if let Ok(site_children) = children.get(site) {
        for child in site_children {
            if let Ok(level) = levels.get(*child) {
                level_map.insert(level.0, *child);
            }
        }
    }

    let mut errors = Vec::new();
    for (id, info) in data {
        let e = subscriber.spawn_scene(info.topic_name);
        let Some(level) = level_map.get(&info.level) else {
            errors.push(SceneLoadingError::MissingLevel(id));
            continue;
        };
        commands.entity(e)
            .insert(info.pose)
            .set_parent(*level);
    }

    if errors.len() == 1 {
        // SAFETY: We already checked that errors has one element
        return Err(errors.pop().unwrap());
    } else if !errors.is_empty() {
        return Err(SceneLoadingError::MultipleErrors(errors));
    }

    Ok(())
}

#[derive(ThisError, Debug)]
enum SceneSavingError {
    #[error("The entity we were told to save is not a valid site")]
    InvalidSite,
    #[error("Failed to serialize: {0}")]
    SerializationFailure(#[from] serde_json::Error),
}

#[derive(ThisError, Debug)]
enum SceneLoadingError {
    #[error("Failed to deserialize: {0}")]
    InvalidSceneData(#[from] serde_json::Error),
    #[error("The level with site id {0} is missing, but it is needed by a scene")]
    MissingLevel(u32),
    #[error("Mutliple errors occurred:\n{0:#?}")]
    MultipleErrors(Vec<SceneLoadingError>),
}

