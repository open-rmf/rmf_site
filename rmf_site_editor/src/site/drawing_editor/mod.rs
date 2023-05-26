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
use crate::site::{Anchor, DrawingMarker};
use crate::{AppState, CurrentWorkspace};

use std::collections::HashSet;

#[derive(Default)]
pub struct DrawingEditorPlugin;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct DrawingEditorHiddenAnchors(HashSet<Entity>);

fn hide_level_entities(
    mut visibilities: Query<&mut Visibility>,
    mut camera_controls: ResMut<CameraControls>,
    mut cameras: Query<&mut Camera>,
    headlight_toggle: Res<HeadlightToggle>,
    mut category_settings: ResMut<VisibilityCategoriesSettings>,
) {
    camera_controls.use_orthographic(true, &mut cameras, &mut visibilities, headlight_toggle.0);
    category_settings.0.doors = false;
    category_settings.0.lanes = false;
    category_settings.0.lifts = false;
    category_settings.0.locations = false;
    category_settings.0.floors = false;
    category_settings.0.models = false;
    category_settings.0.walls = false;
}

fn hide_non_drawing_anchors(
    mut anchors: Query<(Entity, &mut Visibility), With<Anchor>>,
    parents: Query<&Parent>,
    drawings: Query<(), With<DrawingMarker>>,
    mut anchor_set: ResMut<DrawingEditorHiddenAnchors>,
) {
    for (e, mut vis) in &mut anchors {
        if let Ok(parent) = parents.get(e) {
            if drawings.get(**parent).is_err() {
                if vis.is_visible {
                    vis.is_visible = false;
                    anchor_set.insert(e);
                }
            }
        }
    }
}

fn restore_non_drawing_anchors(
    mut visibilities: Query<&mut Visibility>,
    mut anchor_set: ResMut<DrawingEditorHiddenAnchors>,
) {
    for e in anchor_set.drain() {
        visibilities.get_mut(e).map(|mut vis| vis.is_visible = true);
    }
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
            SystemSet::on_enter(AppState::SiteDrawingEditor)
                .with_system(hide_level_entities)
                .with_system(hide_non_drawing_anchors),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::SiteDrawingEditor)
                .with_system(restore_level_entities)
                .with_system(restore_non_drawing_anchors),
        )
        .init_resource::<DrawingEditorHiddenAnchors>();
    }
}
