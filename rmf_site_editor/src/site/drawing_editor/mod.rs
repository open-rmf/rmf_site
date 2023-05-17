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

use crate::interaction::{CameraControls, HeadlightToggle, VisibilityCategoriesSettings};
use crate::AppState;

#[derive(Default)]
pub struct DrawingEditorPlugin;

fn hide_level_entities(
    mut visibilities: Query<&mut Visibility>,
    mut camera_controls: ResMut<CameraControls>,
    mut cameras: Query<&mut Camera>,
    headlight_toggle: Res<HeadlightToggle>,
    mut category_settings: ResMut<VisibilityCategoriesSettings>,
) {
    camera_controls.use_orthographic(true, &mut cameras, &mut visibilities, headlight_toggle.0);
    category_settings.0.lifts = false;
    category_settings.0.floors = false;
    category_settings.0.models = false;
    category_settings.0.measurements = true;
}

fn restore_level_entities(
    mut visibilities: Query<&mut Visibility>,
    mut camera_controls: ResMut<CameraControls>,
    mut cameras: Query<&mut Camera>,
    headlight_toggle: Res<HeadlightToggle>,
    mut category_settings: ResMut<VisibilityCategoriesSettings>,
) {
    camera_controls.use_perspective(true, &mut cameras, &mut visibilities, headlight_toggle.0);
    *category_settings = VisibilityCategoriesSettings::default();
}

impl Plugin for DrawingEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_enter(AppState::SiteDrawingEditor).with_system(hide_level_entities),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::SiteDrawingEditor).with_system(restore_level_entities),
        );
        /*
        app.add_plugin(InfiniteGridPlugin)
            .insert_resource(WgpuSettings {
                features: WgpuFeatures::POLYGON_MODE_LINE,
                ..default()
            })
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
        */
    }
}
