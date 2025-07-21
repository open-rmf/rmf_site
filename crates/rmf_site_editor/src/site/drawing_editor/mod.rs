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

use bevy::{ecs::hierarchy::ChildOf, prelude::*};

pub mod alignment;
pub use alignment::*;
use rmf_site_camera::resources::ProjectionMode;
use rmf_site_picking::Selection;

use crate::AppState;
use crate::{
    interaction::SuppressHighlight,
    site::{DrawingMarker, Edge, MeasurementMarker, NameOfSite, Pending, PreventDeletion},
    CurrentWorkspace, WorkspaceMarker,
};

#[derive(Clone, Copy, Event)]
pub struct BeginEditDrawing(pub Entity);

/// Command to finish editing a drawing. Use None to command any drawing to finish.
#[derive(Clone, Copy, Event)]
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
        let editor = world
            .spawn((Transform::default(), Visibility::default()))
            .id();
        Self {
            editor,
            target: None,
        }
    }
}

impl CurrentEditDrawing {
    pub fn target(&self) -> &Option<EditDrawing> {
        &self.target
    }
}

#[derive(Default)]
pub struct DrawingEditorPlugin;

#[derive(Deref, DerefMut, Event)]
pub struct AlignSiteDrawings(pub Entity);

fn switch_edit_drawing_mode(
    mut commands: Commands,
    mut begin: EventReader<BeginEditDrawing>,
    mut finish: EventReader<FinishEditDrawing>,
    mut current: ResMut<CurrentEditDrawing>,
    mut workspace_visibility: Query<&mut Visibility, With<WorkspaceMarker>>,
    mut app_state: ResMut<NextState<AppState>>,
    mut local_tf: Query<&mut Transform>,
    global_tf: Query<&GlobalTransform>,
    mut projection_mode: ResMut<ProjectionMode>,
    current_workspace: Res<CurrentWorkspace>,
    child_of: Query<&ChildOf, With<DrawingMarker>>,
    is_site: Query<(), With<NameOfSite>>,
) {
    // TODO(@mxgrey): We can make this implementation much cleaner after we
    // update to the latest version of bevy that distinguishes between inherited
    // vs independent visibility.
    //
    // We should also consider using an edit mode stack instead of simply
    // CurrentWorkspace and AppState.
    'handle_begin: {
        if let Some(BeginEditDrawing(e)) = begin.read().last() {
            if current.target().is_some_and(|c| c.drawing == *e) {
                break 'handle_begin;
            }

            if let Some(c) = current.target() {
                // A drawing was being edited and now we're switching to a
                // different drawing, so we need to reset the previous drawing.
                restore_edited_drawing(c, &mut commands);
            }

            let level = if let Ok(co) = child_of.get(*e) {
                co.parent()
            } else {
                error!("Cannot edit {e:?} as a drawing");
                current.target = None;
                break 'handle_begin;
            };

            current.target = Some(EditDrawing { drawing: *e, level });
            commands
                .entity(*e)
                .insert(ChildOf(current.editor))
                .insert(Visibility::Inherited)
                .insert(PreventDeletion::because(
                    "Cannot delete a drawing that is currently being edited".to_owned(),
                ))
                // Highlighting the drawing looks bad when the user will be
                // constantly hovering over it anyway.
                .insert(SuppressHighlight);

            *projection_mode = ProjectionMode::Orthographic;

            if let Ok(mut editor_tf) = local_tf.get_mut(current.editor) {
                if let Ok(level_tf) = global_tf.get(level) {
                    *editor_tf = level_tf.compute_transform();
                } else {
                    error!("Cannot get transform of current level");
                }
            } else {
                error!("Cannot change transform of drawing editor view");
            }

            app_state.set(AppState::SiteDrawingEditor);

            for mut v in &mut workspace_visibility {
                *v = Visibility::Hidden;
            }
        }
    }

    for FinishEditDrawing(finish) in finish.read() {
        let c = if let Some(c) = current.target() {
            if finish.is_some_and(|e| e != c.drawing) {
                continue;
            }
            c
        } else {
            continue;
        };

        restore_edited_drawing(c, &mut commands);
        current.target = None;

        // This camera change would not be needed if we have an edit mode stack
        *projection_mode = ProjectionMode::Perspective;

        if let Some(w) = current_workspace.root {
            if let Ok(mut v) = workspace_visibility.get_mut(w) {
                *v = if current_workspace.display {
                    Visibility::Inherited
                } else {
                    Visibility::Hidden
                };
            }

            if is_site.contains(w) {
                app_state.set(AppState::SiteEditor);
            } else {
                // This logic can probably be improved with an editor mode stack
                error!(
                    "Unable to identify the type for the current workspace \
                    {w:?}, so we will default to site editing mode",
                );
                app_state.set(AppState::SiteEditor);
            }
        }
    }
}

/// Restore a drawing that was being edited back to its normal place and behavior
fn restore_edited_drawing(edit: &EditDrawing, commands: &mut Commands) {
    commands
        .entity(edit.drawing)
        .insert(ChildOf(edit.level))
        .remove::<PreventDeletion>()
        .remove::<SuppressHighlight>();
}

fn assign_drawing_parent_to_new_measurements(
    mut commands: Commands,
    changed_measurement: Query<
        (Entity, &Edge),
        (Without<Pending>, (With<MeasurementMarker>, Changed<Edge>)),
    >,
    child_of: Query<&ChildOf>,
) {
    for (e, edge) in &changed_measurement {
        if let (Ok(p0), Ok(p1)) = (child_of.get(*edge.left()), child_of.get(*edge.right())) {
            if p0.parent() != p1.parent() {
                warn!(
                    "Mismatch in parents of anchors for measurement {e:?}: {:?}, {:?}",
                    p0, p1
                );
            } else {
                commands.entity(e).insert(ChildOf(p0.parent()));
            }
        } else {
            warn!(
                "Missing parents of anchors for measurement {e:?}: {:?}, {:?}",
                child_of.get(*edge.left()),
                child_of.get(*edge.right()),
            );
        }
    }
}

fn make_drawing_default_selected(
    mut selection: ResMut<Selection>,
    current: Res<CurrentEditDrawing>,
) {
    if selection.is_changed() {
        if selection.0.is_none() {
            if let Some(c) = current.target() {
                let drawing_entity = c.drawing;
                selection.0 = Some(drawing_entity);
            } else {
                error!("No drawing while spawning drawing anchor");
            };
        }
    }
}

impl Plugin for DrawingEditorPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<BeginEditDrawing>()
            .add_event::<FinishEditDrawing>()
            .add_event::<AlignSiteDrawings>()
            .add_systems(Update, switch_edit_drawing_mode)
            .add_systems(
                Update,
                (
                    assign_drawing_parent_to_new_measurements,
                    make_drawing_default_selected,
                )
                    .run_if(in_state(AppState::SiteDrawingEditor)),
            );
    }
}
