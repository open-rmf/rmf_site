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
use crate::AppState;
use crate::site::{ChangeCurrentSite, LoadSite};
use crate::workcell::{ChangeCurrentWorkcell, LoadWorkcell};
use rmf_site_format::{Site, SiteProperties, Workcell};

/// Used as an event to command that a new workspace should be made the current one
#[derive(Clone, Copy, Debug)]
pub struct ChangeCurrentWorkspace {
    /// What should the current site be
    pub root: Entity,
}

/// Used as an event to command that a new workspace should be created, behavior will depend on
/// what app mode the editor is currently in
pub struct CreateNewWorkspace;

/// Used as a resource that keeps track of the current workspace
#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct CurrentWorkspace {
    pub root: Option<Entity>,
    pub display: bool,
}

/// Used to keep track of visibility when switching workspace
#[derive(Debug, Default, Resource)]
pub struct RecallWorkspace(Option<Entity>);

impl CurrentWorkspace {
    pub fn to_site(self, open_sites: &Query<Entity, With<SiteProperties>>) -> Option<Entity> {
        let site_entity = self.root?;
        open_sites.get(site_entity).ok()
    }
}

pub struct WorkspacePlugin;

impl Plugin for WorkspacePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ChangeCurrentWorkspace>()
           .add_event::<CreateNewWorkspace>()
           .init_resource::<CurrentWorkspace>()
           .init_resource::<RecallWorkspace>()
           .add_system(dispatch_new_workspace_events)
           //.add_system(dispatch_change_workspace_events)
           .add_system(sync_workspace_visibility);
    }
}

pub fn dispatch_new_workspace_events(
    mut commands: Commands,
    state: Res<State<AppState>>,
    mut new_workspace: EventReader<CreateNewWorkspace>,
    mut load_site: EventWriter<LoadSite>,
    mut load_workcell: EventWriter<LoadWorkcell>,
) {
    if let Some(cmd) = new_workspace.iter().last() {
        match state.current() {
            AppState::MainMenu => {
                println!("DEV ERROR: Sent generic change workspace while in main menu");
            },
            AppState::SiteEditor => {
                load_site.send(LoadSite { site: Site {
                    format_version: Default::default(),
                    anchors: Default::default(),
                    properties: SiteProperties {name: "new_site".to_string()},
                    levels: Default::default(),
                    lifts: Default::default(),
                    navigation: Default::default(),
                    agents: Default::default(),
                }, focus: true, default_file: None });
            },
            AppState::WorkcellEditor => {
                load_workcell.send(LoadWorkcell { workcell: Workcell::default(), focus: true, default_file: None });
            },
        }
    }
}

/*
pub fn dispatch_change_workspace_events(
    mut commands: Commands,
    state: Res<State<AppState>>,
    mut change_workspace: EventReader<ChangeCurrentWorkspace>,
    mut change_site: EventWriter<ChangeCurrentSite>,
    mut change_workcell: EventWriter<ChangeCurrentWorkcell>,
) {
    if let Some(cmd) = change_workspace.iter().last() {
        match state.current() {
            AppState::MainMenu => {
                println!("DEV ERROR: Sent generic change workspace while in main menu");
            },
            AppState::SiteEditor => {
                change_site.send(ChangeCurrentSite { site: cmd.root, level: None });
            },
            AppState::WorkcellEditor => {
                change_workcell.send(ChangeCurrentWorkcell { root: cmd.root });
            },
        }
    }
}
*/

pub fn sync_workspace_visibility(
    current_workspace: Res<CurrentWorkspace>,
    mut recall: ResMut<RecallWorkspace>,
    mut visibility: Query<&mut Visibility>,
) {
    if !current_workspace.is_changed() {
        return;
    }

    if recall.0 != current_workspace.root {
        // Set visibility of current to target
        if let Some(current_workspace_entity) = current_workspace.root {
            if let Ok(mut v) = visibility.get_mut(current_workspace_entity) {
                v.is_visible = current_workspace.display;
            }
        }
        // Disable visibility in recall
        if let Some(recall) = recall.0 {
            if let Ok(mut v) = visibility.get_mut(recall) {
                v.is_visible = false;
            }
        }
        recall.0 = current_workspace.root;
    }
}
