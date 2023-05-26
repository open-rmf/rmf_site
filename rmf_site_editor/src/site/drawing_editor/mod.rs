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

use crate::interaction::{
    CameraControls, HeadlightToggle, Selection, VisibilityCategoriesSettings,
};
use crate::site::{Anchor, DrawingMarker, FiducialMarker, MeasurementMarker, Pending};
use crate::{AppState, CurrentWorkspace};

use std::collections::HashSet;

#[derive(Default)]
pub struct DrawingEditorPlugin;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct DrawingEditorHiddenEntities(HashSet<Entity>);

fn hide_level_entities(
    mut visibilities: Query<&mut Visibility>,
    mut camera_controls: ResMut<CameraControls>,
    mut cameras: Query<&mut Camera>,
    headlight_toggle: Res<HeadlightToggle>,
    mut category_settings: ResMut<VisibilityCategoriesSettings>,
) {
    camera_controls.use_orthographic(true, &mut cameras, &mut visibilities, headlight_toggle.0);
    category_settings.0.constraints = false;
    category_settings.0.doors = false;
    category_settings.0.lanes = false;
    category_settings.0.lifts = false;
    category_settings.0.locations = false;
    category_settings.0.floors = false;
    category_settings.0.models = false;
    category_settings.0.walls = false;
}

fn hide_non_drawing_entities(
    mut anchors: Query<(Entity, &mut Visibility), (With<Anchor>, Without<DrawingMarker>)>,
    parents: Query<&Parent>,
    mut drawings: Query<(Entity, &mut Visibility), (Without<Anchor>, With<DrawingMarker>)>,
    mut anchor_set: ResMut<DrawingEditorHiddenEntities>,
    selection: Res<Selection>,
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
    for (e, mut vis) in &mut drawings {
        if **selection != Some(e) {
            if vis.is_visible {
                vis.is_visible = false;
                anchor_set.insert(e);
            }
        }
    }
}

fn restore_non_drawing_entities(
    mut visibilities: Query<&mut Visibility>,
    mut anchor_set: ResMut<DrawingEditorHiddenEntities>,
) {
    for e in anchor_set.drain() {
        visibilities
            .get_mut(e)
            .map(|mut vis| vis.is_visible = true)
            .ok();
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

fn assign_drawing_parent_to_new_measurements_and_fiducials(
    mut commands: Commands,
    new_elements: Query<
        (Entity, Option<&Parent>),
        (
            Without<Pending>,
            Or<(Added<MeasurementMarker>, Added<FiducialMarker>)>,
        ),
    >,
    drawings: Query<(Entity, &Visibility), With<DrawingMarker>>,
) {
    if new_elements.is_empty() {
        return;
    }
    let parent = match drawings.iter().find(|(_, vis)| vis.is_visible == true) {
        Some(parent) => parent.0,
        None => return,
    };
    for (e, old_parent) in &new_elements {
        // TODO(luca) compute transform here
        if old_parent.map(|p| drawings.get(**p).ok()).is_none() {
            println!("New entity detected");
            commands.entity(parent).add_child(e);
        }
    }
}

impl Plugin for DrawingEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_system_set(
            SystemSet::on_enter(AppState::SiteDrawingEditor)
                .with_system(hide_level_entities)
                .with_system(hide_non_drawing_entities),
        )
        .add_system_set(
            SystemSet::on_exit(AppState::SiteDrawingEditor)
                .with_system(restore_level_entities)
                .with_system(restore_non_drawing_entities),
        )
        .add_system_set(
            SystemSet::on_update(AppState::SiteDrawingEditor)
                .with_system(assign_drawing_parent_to_new_measurements_and_fiducials),
        )
        .init_resource::<DrawingEditorHiddenEntities>();
    }
}
