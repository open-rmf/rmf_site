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

use bevy::prelude::*;

use crate::WorkspaceSaver;

use crate::{
    site::{DrawingMarker, ModelLoadingState},
    Autoload, WorkspaceLoader,
};
use rmf_site_format::NameOfSite;

/// Manages a simple state machine where we:
///   * Wait for a few iterations,
///   * Make sure the world is loaded.
///   * Send a save event.
///   * Wait for a few iterations.
///   * Exit.
//
// TODO(@mxgrey): Introduce a "workspace has finished loading" event, and create
// a workflow that reacts to that event.
#[derive(Debug, Resource)]
pub struct HeadlessExportState {
    iterations: u32,
    world_loaded: bool,
    save_requested: bool,
    sdf_target_path: Option<String>,
    nav_target_path: Option<String>,
}

impl HeadlessExportState {
    pub fn new(sdf_target_path: Option<String>, nav_target_path: Option<String>) -> Self {
        Self {
            iterations: 0,
            world_loaded: false,
            save_requested: false,
            sdf_target_path,
            nav_target_path,
        }
    }
}

pub fn headless_export(
    mut commands: Commands,
    mut workspace_saver: WorkspaceSaver,
    mut exit: EventWriter<bevy::app::AppExit>,
    missing_models: Query<(), With<ModelLoadingState>>,
    mut export_state: ResMut<HeadlessExportState>,
    sites: Query<(Entity, &NameOfSite)>,
    drawings: Query<Entity, With<DrawingMarker>>,
    autoload: Option<ResMut<Autoload>>,
    mut workspace_loader: WorkspaceLoader,
) {
    if let Some(mut autoload) = autoload {
        if let Some(filename) = autoload.filename.take() {
            workspace_loader.load_from_path(filename);
        }
    } else {
        error!("Cannot perform a headless export since no site file was specified for loading");
        exit.write(bevy::app::AppExit::error());
        return;
    }

    export_state.iterations += 1;
    if export_state.iterations < 5 {
        return;
    }
    if sites.is_empty() {
        error!("No site is loaded so we cannot export anything");
        exit.write(bevy::app::AppExit::error());
    }
    if !missing_models.is_empty() {
        // Despawn all drawings, otherwise floors will become transparent.
        for e in drawings.iter() {
            commands.entity(e).despawn();
        }
        // TODO(luca) implement a timeout logic?
    } else {
        if !export_state.world_loaded {
            export_state.iterations = 0;
            export_state.world_loaded = true;
        } else {
            if !export_state.save_requested && export_state.iterations > 5 {
                if let Some(sdf_target_path) = &export_state.sdf_target_path {
                    let path = std::path::PathBuf::from(sdf_target_path.clone());
                    workspace_saver.export_sdf_to_path(path);
                }

                if let Some(nav_target_path) = &export_state.nav_target_path {
                    let path = std::path::PathBuf::from(nav_target_path.clone());
                    workspace_saver.export_nav_graphs_to_path(path);
                }

                export_state.save_requested = true;
                export_state.iterations = 0;
            } else if export_state.save_requested && export_state.iterations > 5 {
                exit.write(bevy::app::AppExit::Success);
            }
        }
    }
}
