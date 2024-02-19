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
use crate::widgets::{
    diagnostic_window::DiagnosticWindowState,
    menu_bar::{MenuEvent, MenuItem, MenuVisualizationStates, ToolMenu},
};
use crate::AppState;
use bevy::prelude::*;
use bevy::utils::{HashMap, Uuid};
use rmf_site_format::{FilteredIssueKinds, FilteredIssues, IssueKey};
use std::collections::HashSet;

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
            .world
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

#[derive(Resource)]
pub struct IssueMenu {
    diagnostic_tool: Entity,
}

impl FromWorld for IssueMenu {
    fn from_world(world: &mut World) -> Self {
        let target_states = HashSet::from([
            AppState::SiteEditor,
            AppState::SiteDrawingEditor,
            AppState::SiteVisualizer,
        ]);
        // Tools menu
        let diagnostic_tool = world
            .spawn(MenuItem::Text("Diagnostic Tool".to_string()))
            .insert(MenuVisualizationStates(target_states))
            .id();

        let tool_header = world.resource::<ToolMenu>().get();
        world
            .entity_mut(tool_header)
            .push_children(&[diagnostic_tool]);

        IssueMenu { diagnostic_tool }
    }
}

fn handle_diagnostic_window_visibility(
    mut menu_events: EventReader<MenuEvent>,
    issue_menu: Res<IssueMenu>,
    mut diagnostic_window: ResMut<DiagnosticWindowState>,
) {
    for event in menu_events.read() {
        if event.clicked() && event.source() == issue_menu.diagnostic_tool {
            diagnostic_window.show = true;
        }
    }
}

impl Plugin for IssuePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ValidateWorkspace>()
            .add_plugins((
                ChangePlugin::<FilteredIssues<Entity>>::default(),
                ChangePlugin::<FilteredIssueKinds>::default(),
            ))
            .init_resource::<IssueDictionary>()
            .init_resource::<IssueMenu>()
            .add_systems(Update, handle_diagnostic_window_visibility);
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
