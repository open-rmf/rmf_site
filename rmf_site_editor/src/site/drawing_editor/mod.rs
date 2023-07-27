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

use bevy::{prelude::*, render::view::visibility::RenderLayers};

pub mod optimizer;
pub use optimizer::*;

use crate::{
    interaction::{CameraControls, HeadlightToggle, Selection, ChangeProjectionMode},
    site::{
        Anchor, DrawingMarker, Edge, FiducialMarker, MeasurementMarker, Pending,
        PixelsPerMeter, Point, PreventDeletion, SiteProperties, WorkcellProperties,
    },
    WorkspaceMarker, CurrentWorkspace,
};
use crate::AppState;

use std::collections::HashSet;

#[derive(Clone, Copy)]
pub struct BeginEditDrawing(pub Entity);

/// Command to finish editing a drawing. Use None to command any drawing to finish.
#[derive(Clone, Copy)]
pub struct FinishEditDrawing(pub Option<Entity>);

#[derive(Clone, Copy)]
pub struct EditDrawing {
    /// What drawing is being edited
    pub drawing: Entity,
    /// What is the original parent level for the drawing
    pub level: Entity,
}

#[derive(Clone, Copy, Resource)]
pub struct CurrentEditDrawing {
    editor: Entity,
    target: Option<EditDrawing>,
}

impl FromWorld for CurrentEditDrawing {
    fn from_world(world: &mut World) -> Self {
        let editor = world.spawn(SpatialBundle::default()).id();
        Self { editor, target: None }
    }
}

impl CurrentEditDrawing {
    pub fn target(&self) -> &Option<EditDrawing> {
        &self.target
    }
}

#[derive(Default)]
pub struct DrawingEditorPlugin;

#[derive(Resource, Default, Deref, DerefMut)]
pub struct DrawingEditorHiddenEntities(HashSet<Entity>);

// TODO(luca) should these events be defined somewhere else?
#[derive(Deref, DerefMut)]
pub struct ScaleDrawing(pub Entity);

#[derive(Deref, DerefMut)]
pub struct AlignLevelDrawings(pub Entity);

#[derive(Deref, DerefMut)]
pub struct AlignSiteDrawings(pub Entity);

fn switch_edit_drawing_mode(
    mut commands: Commands,
    mut begin: EventReader<BeginEditDrawing>,
    mut finish: EventReader<FinishEditDrawing>,
    mut current: ResMut<CurrentEditDrawing>,
    mut workspace_visibility: Query<&mut Visibility, With<WorkspaceMarker>>,
    mut app_state: ResMut<State<AppState>>,
    mut local_tf: Query<&mut Transform>,
    mut change_camera_mode: EventWriter<ChangeProjectionMode>,
    global_tf: Query<&GlobalTransform>,
    current_workspace: Res<CurrentWorkspace>,
    parent: Query<&Parent, With<DrawingMarker>>,
    is_site: Query<(), With<SiteProperties>>,
    is_workcell: Query<(), With<WorkcellProperties>>,
) {
    // TODO(@mxgrey): We can make this implementation much cleaner after we
    // update to the latest version of bevy that distinguishes between inherited
    // vs independent visibility.
    //
    // We should also consider using an edit mode stack instead of simply
    // CurrentWorkspace and AppState.
    'handle_begin: {
        if let Some(BeginEditDrawing(e)) = begin.iter().last() {
            if current.target().is_some_and(|c| c.drawing == *e) {
                break 'handle_begin;
            }

            if let Some(c) = current.target() {
                // A drawing was being edited and now we're switching to a
                // different drawing, so we need to reset the previous drawing.
                commands.entity(c.drawing)
                    .set_parent(c.level)
                    .remove::<PreventDeletion>();
            }

            let level = if let Ok(p) = parent.get(*e) {
                p.get()
            } else {
                error!("Cannot edit {e:?} as a drawing");
                current.target = None;
                break 'handle_begin;
            };

            current.target = Some(EditDrawing { drawing: *e, level });
            commands.entity(*e)
                .set_parent(current.editor)
                .insert(Visibility { is_visible: true })
                .insert(ComputedVisibility::default())
                .insert(PreventDeletion::because(
                    "Cannot delete a drawing that is currently being edited"
                    .to_owned()
                ));

            change_camera_mode.send(ChangeProjectionMode::to_orthographic());

            if let Ok(mut editor_tf) = local_tf.get_mut(current.editor) {
                if let Ok(mut level_tf) = global_tf.get(level) {
                    *editor_tf = level_tf.compute_transform();
                } else {
                    error!("Cannot get transform of current level");
                }
            } else {
                error!("Cannot change transform of drawing editor view");
            }

            if let Some(err) = app_state.set(AppState::SiteDrawingEditor).err() {
                error!("Unable to switch to drawing editor mode: {err:?}");
            }

            for mut v in &mut workspace_visibility {
                v.is_visible = false;
            }
        }
    }

    for FinishEditDrawing(finish) in finish.iter() {
        let c = if let Some(c) = current.target() {
            if finish.is_some_and(|e| e != c.drawing) {
                continue;
            }
            c
        } else {
            continue;
        };

        commands.entity(c.drawing)
            .set_parent(c.level)
            .remove::<PreventDeletion>();
        current.target = None;

        // This camera change would not be needed if we have an edit mode stack
        change_camera_mode.send(ChangeProjectionMode::to_perspective());

        if let Some(w) = current_workspace.root {
            if let Ok(mut v) = workspace_visibility.get_mut(w) {
                v.is_visible = current_workspace.display;
            }

            if is_site.contains(w) {
                if let Some(err) = app_state.set(AppState::SiteEditor).err() {
                    error!("Failed to switch back to site editing mode: {err:?}");
                }
            } else if is_workcell.contains(w) {
                if let Some(err) = app_state.set(AppState::WorkcellEditor).err() {
                    error!("Failed to switch back to workcell editing mode: {err:?}");
                }
            } else {
                // This logic can probably be improved with an editor mode stack
                error!(
                    "Unable to identify the type for the current workspace \
                    {w:?}, so we will default to site editing mode",
                );
                if let Some(err) = app_state.set(AppState::SiteEditor).err() {
                    error!("Failed to switch back to site editing mode: {err:?}");
                }
            }
        }
    }
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
        app
            .add_event::<ScaleDrawing>()
            .add_event::<BeginEditDrawing>()
            .add_event::<FinishEditDrawing>()
            .init_resource::<CurrentEditDrawing>()
            .add_system(switch_edit_drawing_mode)
            .add_system_set(
                SystemSet::on_update(AppState::SiteDrawingEditor)
                    .with_system(assign_drawing_parent_to_new_measurements_and_fiducials)
                    .with_system(scale_drawings)
                    .with_system(make_drawing_default_selected),
            )
            .init_resource::<DrawingEditorHiddenEntities>();
    }
}
