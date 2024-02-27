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

/// Used as an event to command that a new workspace should be created, behavior will depend on
/// what app mode the editor is currently in
/// Users should react to this event and create a new workspace for their required app mode.
#[derive(Event)]
pub struct CreateNewWorkspace;

/// Apply this component to all workspace types
#[derive(Component)]
pub struct WorkspaceMarker;

/// Used as an event to command that a workspace should be loaded. This will spawn a file open
/// dialog (in non-wasm) with allowed extensions depending on the app state
/// Users should react to this event and spawn a dialog with their required filters.
#[derive(Event)]
pub enum LoadWorkspace {
    Dialog,
}

/// Used as a resource that keeps track of the current workspace
// TODO(@mxgrey): Consider a workspace stack, e.g. so users can temporarily edit
// a workcell inside of a site and then revert back into the site.
#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct CurrentWorkspace {
    pub root: Option<Entity>,
    pub display: bool,
}

#[derive(Event)]
pub struct SaveWorkspace {
    /// If specified workspace will be saved to requested file, otherwise the default file
    pub destination: SaveWorkspaceDestination,
}

impl SaveWorkspace {
    pub fn new() -> Self {
        Self {
            destination: SaveWorkspaceDestination::default(),
        }
    }

    pub fn to_default_file(mut self) -> Self {
        self.destination = SaveWorkspaceDestination::DefaultFile;
        self
    }

    pub fn to_dialog(mut self) -> Self {
        self.destination = SaveWorkspaceDestination::Dialog;
        self
    }
}

#[derive(Default, Debug, Clone)]
pub enum SaveWorkspaceDestination {
    #[default]
    DefaultFile,
    Dialog,
}

#[derive(Event, Default)]
/// Export the workspace to an application dependent type.
/// Users should implement their own EventReader to parse it.
pub struct ExportWorkspace;

/// Used to keep track of visibility when switching workspace
#[derive(Debug, Default, Resource)]
pub struct RecallWorkspace(Option<Entity>);

pub struct WorkspacePlugin;

impl Plugin for WorkspacePlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<CreateNewWorkspace>()
            .add_event::<LoadWorkspace>()
            .add_event::<SaveWorkspace>()
            .add_event::<ExportWorkspace>()
            .init_resource::<CurrentWorkspace>()
            .init_resource::<RecallWorkspace>()
            .add_systems(Update, sync_workspace_visibility);
    }
}

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
                *v = if current_workspace.display {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }
        }
        // Disable visibility in recall
        if let Some(recall) = recall.0 {
            if let Ok(mut v) = visibility.get_mut(recall) {
                *v = Visibility::Hidden;
            }
        }
        recall.0 = current_workspace.root;
    }
}
