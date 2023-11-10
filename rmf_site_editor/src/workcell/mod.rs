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

pub mod load;
pub use load::*;

pub mod keyboard;
pub use keyboard::*;

pub mod mesh_constraint;
pub use mesh_constraint::*;

pub mod save;
pub use save::*;

pub mod workcell;
pub use workcell::*;

use bevy::render::{render_resource::WgpuFeatures, settings::WgpuSettings};
use bevy::{prelude::*, render::view::visibility::VisibilitySystems, transform::TransformSystem};
use bevy_infinite_grid::{InfiniteGrid, InfiniteGridBundle, InfiniteGridPlugin};

use crate::interaction::Gizmo;
use crate::AppState;
use crate::{
    shapes::make_infinite_grid,
    site::{
        clear_model_trashcan, handle_model_loaded_events, handle_new_mesh_primitives,
        handle_new_sdf_roots, handle_update_fuel_cache_requests, make_models_selectable,
        propagate_model_render_layers, read_update_fuel_cache_results,
        reload_failed_models_with_new_api_key, update_anchor_transforms, update_model_scales,
        update_model_scenes, update_model_tentative_formats, update_transforms_for_changed_poses,
    },
};

use rmf_site_format::ModelMarker;

use bevy_rapier3d::prelude::*;

#[derive(Default)]
pub struct WorkcellEditorPlugin;

fn spawn_grid(mut commands: Commands) {
    commands.spawn(make_infinite_grid(1.0, 100.0, None));
}

fn delete_grid(mut commands: Commands, grids: Query<Entity, With<InfiniteGrid>>) {
    for grid in grids.iter() {
        commands.entity(grid).despawn_recursive();
    }
}

impl Plugin for WorkcellEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InfiniteGridPlugin)
            .add_event::<SaveWorkcell>()
            .add_event::<LoadWorkcell>()
            .add_event::<ChangeCurrentWorkcell>()
            .add_systems(OnEnter(AppState::WorkcellEditor), spawn_grid)
            .add_systems(OnExit(AppState::WorkcellEditor), delete_grid)
            .add_systems(
                Update,
                (
                    update_constraint_dependents,
                    handle_model_loaded_events,
                    update_model_scenes,
                    update_model_scales,
                    update_model_tentative_formats,
                    propagate_model_render_layers,
                    make_models_selectable,
                    handle_update_fuel_cache_requests,
                    read_update_fuel_cache_results,
                    reload_failed_models_with_new_api_key,
                    handle_workcell_keyboard_input,
                    handle_new_mesh_primitives,
                    change_workcell.before(load_workcell),
                    handle_new_sdf_roots,
                )
                    .run_if(in_state(AppState::WorkcellEditor)),
            )
            .add_systems(
                PreUpdate,
                clear_model_trashcan.run_if(in_state(AppState::WorkcellEditor)),
            )
            .add_systems(
                Update,
                (load_workcell, save_workcell, add_workcell_visualization),
            )
            // TODO(luca) restore doing this before transform propagation
            .add_systems(
                Update,
                (
                    update_anchor_transforms,
                    add_anchors_for_new_mesh_constraints,
                    update_transforms_for_changed_poses,
                )
                    .run_if(in_state(AppState::WorkcellEditor)),
            );
    }
}
