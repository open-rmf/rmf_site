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

mod optimizer;
use optimizer::*;

use crate::interaction::{
    CameraControls, HeadlightToggle, Selection, VisibilityCategoriesSettings,
};
use crate::site::{
    Anchor, DrawingMarker, Edge, FiducialMarker, MeasurementMarker, Pending, PixelsPerMeter, Point,
};
use crate::AppState;

use std::collections::HashSet;

#[derive(Default)]
pub struct DrawingEditorPlugin;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct DrawingEditorHiddenEntities(HashSet<Entity>);

#[derive(Deref, DerefMut)]
pub struct ScaleDrawing(pub Entity);

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
    mut new_elements: Query<
        (Entity, Option<&Parent>, &mut Transform),
        (
            Without<Pending>,
            Or<(
                (With<MeasurementMarker>, Changed<Edge<Entity>>),
                (Changed<Point<Entity>>, With<FiducialMarker>),
            )>,
        ),
    >,
    drawings: Query<(Entity, &Visibility, &PixelsPerMeter), With<DrawingMarker>>,
) {
    if new_elements.is_empty() {
        return;
    }
    let (parent, ppm) = match drawings.iter().find(|(_, vis, _)| vis.is_visible == true) {
        Some(parent) => (parent.0, parent.2),
        None => return,
    };
    for (e, old_parent, mut tf) in &mut new_elements {
        if old_parent.map(|p| drawings.get(**p).ok()).is_none() {
            commands.entity(parent).add_child(e);
            // Set its scale to the parent's pixels per meter to make it in pixel coordinates
            tf.scale = Vec3::new(ppm.0, ppm.0, 1.0);
        }
    }
}

fn make_drawing_default_selected(
    drawings: Query<(Entity, &Visibility), With<DrawingMarker>>,
    mut selection: ResMut<Selection>,
) {
    if selection.is_changed() {
        if selection.0.is_none() {
            if let Some(drawing) = drawings.iter().find(|(_, vis)| vis.is_visible == true) {
                selection.0 = Some(drawing.0);
            }
        }
    }
}

impl Plugin for DrawingEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<ScaleDrawing>()
            .add_system_set(
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
                    .with_system(assign_drawing_parent_to_new_measurements_and_fiducials)
                    .with_system(scale_drawings)
                    .with_system(make_drawing_default_selected),
            )
            .init_resource::<DrawingEditorHiddenEntities>();
    }
}
