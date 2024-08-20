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

pub mod frame;
pub use frame::*;

pub mod joint;
pub use joint::*;

pub mod load;
pub use load::*;

pub mod keyboard;
pub use keyboard::*;

pub mod menu;
pub use menu::*;

pub mod model;
pub use model::*;

pub mod save;
pub use save::*;

pub mod urdf_package_exporter;

pub mod workcell;
pub use workcell::*;

use bevy::prelude::*;
use bevy_infinite_grid::{InfiniteGrid, InfiniteGridPlugin};

use crate::AppState;
use crate::{
    shapes::make_infinite_grid,
    site::{
        clear_model_trashcan, handle_model_loaded_events, handle_new_primitive_shapes,
        handle_update_fuel_cache_requests, make_models_selectable, propagate_model_properties,
        read_update_fuel_cache_results, reload_failed_models_with_new_api_key,
        update_anchor_transforms, update_model_scales, update_model_scenes,
        update_model_tentative_formats, update_transforms_for_changed_poses,
    },
};

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
        app.add_plugins(InfiniteGridPlugin)
            .add_event::<CreateJoint>()
            .add_event::<ChangeCurrentWorkcell>()
            .add_systems(OnEnter(AppState::WorkcellEditor), spawn_grid)
            .add_systems(OnExit(AppState::WorkcellEditor), delete_grid)
            .add_systems(
                Update,
                (
                    (
                        propagate_model_properties,
                        make_models_selectable,
                        // Counter-intuitively, we want to expand the model scenes
                        // after propagating the model properties through the scene.
                        // The reason is that the entities for the scene won't be
                        // available until the next cycle is finished, so we want to
                        // wait until the next cycle before propagating the properties
                        // of any newly added scenes.
                        handle_model_loaded_events,
                    ).chain(),
                    update_model_scenes,
                    update_model_scales,
                    handle_new_primitive_shapes,
                    update_model_tentative_formats,
                    replace_name_in_site_components,
                    handle_create_joint_events,
                    cleanup_orphaned_joints,
                    handle_update_fuel_cache_requests,
                    read_update_fuel_cache_results,
                    reload_failed_models_with_new_api_key,
                    handle_workcell_keyboard_input,
                    change_workcell.before(load_workcell),
                    handle_export_urdf_menu_events,
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
                    scale_workcell_anchors,
                    update_anchor_transforms,
                    update_transforms_for_changed_poses,
                )
                    .run_if(in_state(AppState::WorkcellEditor)),
            )
            .add_systems(
                PostUpdate,
                (flatten_loaded_models_hierarchy,).run_if(in_state(AppState::WorkcellEditor)),
            );
    }

    // Put the UI dependent plugins in `finish` to make sure the interaction is initialized first
    fn finish(&self, app: &mut App) {
        app.init_resource::<ExportUrdfMenu>();
    }
}
