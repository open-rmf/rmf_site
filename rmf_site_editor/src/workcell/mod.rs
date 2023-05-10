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

pub mod urdf;
pub use urdf::*;

use bevy::pbr::wireframe::{Wireframe, WireframePlugin};
use bevy::render::{render_resource::WgpuFeatures, settings::WgpuSettings};
use bevy::{prelude::*, render::view::visibility::VisibilitySystems, transform::TransformSystem};
use bevy_infinite_grid::{InfiniteGrid, InfiniteGridBundle, InfiniteGridPlugin};

use crate::interaction::Gizmo;
use crate::AppState;
use crate::{
    shapes::make_infinite_grid,
    site::{
        handle_new_mesh_primitives, make_models_selectable, update_anchor_transforms,
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

fn add_wireframe_to_meshes(
    mut commands: Commands,
    new_meshes: Query<Entity, Added<Handle<Mesh>>>,
    parents: Query<&Parent>,
    models: Query<Entity, With<ModelMarker>>,
) {
    for e in new_meshes.iter() {
        for ancestor in AncestorIter::new(&parents, e) {
            if let Ok(_) = models.get(ancestor) {
                println!("Adding wireframe to mesh {:?}", e);
                commands.entity(e).insert(Wireframe);
            }
        }
    }
}

impl Plugin for WorkcellEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugin(InfiniteGridPlugin)
            .insert_resource(WgpuSettings {
                features: WgpuFeatures::POLYGON_MODE_LINE,
                ..default()
            })
            .add_plugin(WireframePlugin)
            .add_plugin(RapierPhysicsPlugin::<NoUserData>::default())
            .add_plugin(RapierDebugRenderPlugin::default())
            .add_event::<SaveWorkcell>()
            .add_event::<LoadWorkcell>()
            .add_event::<ChangeCurrentWorkcell>()
            .add_system_set(SystemSet::on_enter(AppState::WorkcellEditor).with_system(spawn_grid))
            .add_system_set(SystemSet::on_exit(AppState::WorkcellEditor).with_system(delete_grid))
            .add_system_set(
                SystemSet::on_update(AppState::WorkcellEditor)
                    .with_system(add_wireframe_to_meshes)
                    .with_system(update_constraint_dependents)
                    .with_system(update_model_scenes)
                    .with_system(update_model_tentative_formats)
                    .with_system(make_models_selectable)
                    .with_system(handle_workcell_keyboard_input)
                    .with_system(handle_new_mesh_primitives)
                    .with_system(change_workcell.before(load_workcell))
                    .with_system(handle_new_urdf_roots),
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
