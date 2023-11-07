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

pub mod joint;
pub use joint::*;

pub mod load;
pub use load::*;

pub mod keyboard;
pub use keyboard::*;

pub mod mesh_constraint;
pub use mesh_constraint::*;

pub mod model;
pub use model::*;

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
        clear_model_trashcan, handle_model_loaded_events, handle_new_primitive_shapes,
        handle_new_sdf_roots, handle_update_fuel_cache_requests, make_models_selectable,
        propagate_model_render_layers, read_update_fuel_cache_results,
        reload_failed_models_with_new_api_key, update_anchor_transforms, update_model_scales,
        update_model_scenes, update_model_tentative_formats, update_transforms_for_changed_poses,
    },
};

use rmf_site_format::ModelMarker;

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
            .insert_resource(WgpuSettings {
                features: WgpuFeatures::POLYGON_MODE_LINE,
                ..default()
            })
            .add_event::<CreateJoint>()
            .add_event::<SaveWorkcell>()
            .add_event::<LoadWorkcell>()
            .add_event::<ChangeCurrentWorkcell>()
            .add_system_set(SystemSet::on_enter(AppState::WorkcellEditor).with_system(spawn_grid))
            .add_system_set(SystemSet::on_exit(AppState::WorkcellEditor).with_system(delete_grid))
            .add_system_set(
                SystemSet::on_update(AppState::WorkcellEditor)
                    .with_system(update_constraint_dependents)
                    .with_system(handle_model_loaded_events)
                    .with_system(update_model_scenes)
                    .with_system(flatten_loaded_models_hierarchy)
                    .with_system(replace_name_in_site_components)
                    .with_system(update_model_scales)
                    .with_system(handle_create_joint_events)
                    .with_system(cleanup_orphaned_joints)
                    .with_system(update_model_tentative_formats)
                    .with_system(propagate_model_render_layers)
                    .with_system(make_models_selectable)
                    .with_system(handle_update_fuel_cache_requests)
                    .with_system(read_update_fuel_cache_results)
                    .with_system(reload_failed_models_with_new_api_key)
                    .with_system(handle_workcell_keyboard_input)
                    .with_system(handle_new_primitive_shapes)
                    .with_system(change_workcell.before(load_workcell))
                    .with_system(handle_new_sdf_roots),
            )
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::on_update(AppState::WorkcellEditor).with_system(clear_model_trashcan),
            )
            .add_system(load_workcell)
            .add_system(save_workcell)
            .add_system(add_workcell_visualization)
            .add_system_set(
                SystemSet::on_update(AppState::WorkcellEditor)
                    .before(TransformSystem::TransformPropagate)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .with_system(update_anchor_transforms)
                    .with_system(
                        add_anchors_for_new_mesh_constraints.before(update_anchor_transforms),
                    )
                    .with_system(update_transforms_for_changed_poses),
            );
    }
}
