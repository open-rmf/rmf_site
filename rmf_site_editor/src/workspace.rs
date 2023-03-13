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
use crate::site::{ChangeCurrentSite};
use crate::workcell::{ChangeCurrentWorkcell};

/// Used as an event to command that a new site should be made the current one
#[derive(Clone, Copy, Debug)]
pub struct ChangeCurrentWorkspace {
    /// What should the current site be
    pub root: Entity,
}

pub struct WorkspacePlugin;

impl Plugin for WorkspacePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ChangeCurrentWorkspace>()
           .add_system(dispatch_change_workspace_events);
    }
}

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
                change_site.send(ChangeCurrentSite { root: cmd.root });
            },
            AppState::WorkcellEditor => {
                change_workcell.send(ChangeCurrentWorkcell { root: cmd.root });
            },
        }
    }
}
