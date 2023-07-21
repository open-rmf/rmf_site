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

use crate::site::SiteProperties;
use crate::CurrentWorkspace;
use bevy::prelude::*;
use bevy::utils::{HashMap, HashSet, Uuid};
use rmf_site_format::IssueKey;

#[derive(Component, Debug, Clone)]
pub struct Issue {
    pub key: IssueKey<Entity>,
    /// Short description of the issue
    pub brief: String,
    /// Hint on how to approach solving the issue
    pub hint: String,
}

/// Used as an event to request validation of the current workspace
pub struct ValidateCurrentWorkspace;

// Maps a uuid to the issue name
#[derive(Default, Resource, Deref, DerefMut)]
pub struct IssueDictionary(HashMap<Uuid, String>);

#[derive(Default)]
pub struct IssuePlugin;

impl Plugin for IssuePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ValidateCurrentWorkspace>()
            .init_resource::<IssueDictionary>();
    }
}

pub fn clear_old_issues_on_new_validate_event(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateCurrentWorkspace>,
    children: Query<&Children>,
    issues: Query<Entity, With<Issue>>,
    current_workspace: Res<CurrentWorkspace>,
) {
    if validate_events.iter().last().is_some() {
        let Some(root) = current_workspace.root else {
            return;
        };
        let Ok(children) = children.get(root) else {
            return;
        };
        for e in children {
            if issues.get(*e).is_ok() {
                commands.entity(*e).despawn_recursive();
            }
        }
    }
}

pub fn register_issue(type_uuid: Uuid, name: String, dictionary: &mut IssueDictionary) {
    if !dictionary.contains_key(&type_uuid) {
        dictionary.insert(type_uuid, name);
    } else {
        warn!(
            "Issue type {} with name {} already registered",
            type_uuid, name
        );
    }
}
