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

use crate::site::ChangePlugin;
use bevy::{prelude::*, utils::HashMap};
use rmf_site_format::{FilteredIssueKinds, FilteredIssues, IssueKey};
use uuid::Uuid;

#[derive(Component, Debug, Clone)]
pub struct Issue {
    pub key: IssueKey<Entity>,
    /// Short description of the issue
    pub brief: String,
    /// Hint on how to approach solving the issue
    pub hint: String,
}

pub trait RegisterIssueType {
    fn add_issue_type(&mut self, type_uuid: &Uuid, name: &str) -> &mut Self;
}

impl RegisterIssueType for App {
    fn add_issue_type(&mut self, type_uuid: &Uuid, name: &str) -> &mut Self {
        let mut issue_dictionary = self
            .world_mut()
            .get_resource_or_insert_with::<IssueDictionary>(Default::default);
        issue_dictionary.insert(type_uuid.clone(), name.into());
        self
    }
}

/// Used as an event to request validation of a workspace
#[derive(Deref, DerefMut, Event)]
pub struct ValidateWorkspace(pub Entity);

// Maps a uuid to the issue name
#[derive(Default, Resource, Deref, DerefMut)]
pub struct IssueDictionary(HashMap<Uuid, String>);

#[derive(Default)]
pub struct IssuePlugin;

impl Plugin for IssuePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ValidateWorkspace>()
            .add_plugins((
                ChangePlugin::<FilteredIssues<Entity>>::default(),
                ChangePlugin::<FilteredIssueKinds>::default(),
            ))
            .init_resource::<IssueDictionary>()
            .add_systems(PostUpdate, clear_old_issues_on_new_validate_event);
    }
}

pub fn clear_old_issues_on_new_validate_event(
    mut commands: Commands,
    mut validate_events: EventReader<ValidateWorkspace>,
    children: Query<&Children>,
    issues: Query<Entity, With<Issue>>,
) {
    for root in validate_events.read() {
        let Ok(children) = children.get(**root) else {
            return;
        };
        for e in children {
            if issues.get(*e).is_ok() {
                commands.entity(*e).despawn_recursive();
            }
        }
    }
}
