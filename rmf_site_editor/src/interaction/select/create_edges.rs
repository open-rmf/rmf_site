/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use crate::{
    interaction::*,
    site::{ChangeDependent, Pending, TextureNeedsAssignment},
};
use bevy::prelude::*;
use bevy_impulse::*;
use rmf_site_format::{Edge, Side};
use std::borrow::Borrow;

pub fn spawn_create_edges_service(
    helpers: &AnchorSelectionHelpers,
    app: &mut App,
) -> Service<Option<Entity>, ()> {
    let anchor_setup =
        app.spawn_service(anchor_selection_setup::<CreateEdges>.into_blocking_service());
    let state_setup = app.spawn_service(create_edges_setup.into_blocking_service());
    let update_preview = app.spawn_service(on_hover_for_create_edges.into_blocking_service());
    let update_current = app.spawn_service(on_select_for_create_edges.into_blocking_service());
    let handle_key_code = app.spawn_service(on_keyboard_for_create_edges.into_blocking_service());
    let cleanup_state = app.spawn_service(cleanup_create_edges.into_blocking_service());

    helpers.spawn_anchor_selection_workflow(
        anchor_setup,
        state_setup,
        update_preview,
        update_current,
        handle_key_code,
        cleanup_state,
        app.world_mut(),
    )
}

pub struct CreateEdges {
    pub spawn_edge: fn(Edge<Entity>, &mut Commands) -> Entity,
    pub preview_edge: Option<PreviewEdge>,
    pub continuity: EdgeContinuity,
    pub scope: AnchorScope,
}

impl CreateEdges {
    pub fn new<T: Bundle + From<Edge<Entity>>>(
        continuity: EdgeContinuity,
        scope: AnchorScope,
    ) -> Self {
        Self {
            spawn_edge: create_edge::<T>,
            preview_edge: None,
            continuity,
            scope,
        }
    }

    pub fn new_with_texture<T: Bundle + From<Edge<Entity>>>(
        continuity: EdgeContinuity,
        scope: AnchorScope,
    ) -> Self {
        Self {
            spawn_edge: create_edge_with_texture::<T>,
            preview_edge: None,
            continuity,
            scope,
        }
    }

    pub fn initialize_preview(&mut self, anchor: Entity, commands: &mut Commands) {
        let edge = Edge::new(anchor, anchor);
        let edge = (self.spawn_edge)(edge, commands);
        self.preview_edge = Some(PreviewEdge {
            edge,
            side: Side::start(),
            provisional_start: false,
        });

        commands.queue(ChangeDependent::add(anchor, edge));
    }
}

impl Borrow<AnchorScope> for CreateEdges {
    fn borrow(&self) -> &AnchorScope {
        &self.scope
    }
}

fn create_edge<T: Bundle + From<Edge<Entity>>>(
    edge: Edge<Entity>,
    commands: &mut Commands,
) -> Entity {
    let new_bundle: T = edge.into();
    commands.spawn((new_bundle, Pending)).id()
}

fn create_edge_with_texture<T: Bundle + From<Edge<Entity>>>(
    edge: Edge<Entity>,
    commands: &mut Commands,
) -> Entity {
    let new_bundle: T = edge.into();
    commands
        .spawn((new_bundle, TextureNeedsAssignment, Pending))
        .id()
}

#[derive(Clone, Copy)]
pub struct PreviewEdge {
    pub edge: Entity,
    pub side: Side,
    /// True if the start anchor of the edge was created specifically to build
    /// this edge. If this true, we will despawn the anchor during cleanup if
    /// the edge does not get completed.
    pub provisional_start: bool,
}

impl PreviewEdge {
    pub fn cleanup(
        &self,
        edges: &Query<&'static Edge<Entity>>,
        commands: &mut Commands,
    ) -> SelectionNodeResult {
        let edge = edges.get(self.edge).or_broken_query()?;
        for anchor in edge.array() {
            commands.queue(ChangeDependent::remove(anchor, self.edge));
        }

        if self.provisional_start {
            // The start anchor was created specifically for this preview edge
            // which we are about to despawn. Let's despawn both so we aren't
            // littering the scene with unintended anchors.
            commands
                .get_entity(edge.start())
                .or_broken_query()?
                .despawn_recursive();
        }

        commands
            .get_entity(self.edge)
            .or_broken_query()?
            .despawn_recursive();
        Ok(())
    }
}

pub enum EdgeContinuity {
    /// Create just a single edge
    Single,
    /// Create a sequence of separate edges
    Separate,
    /// Create edges continuously, i.e. the beginning of the next edge will
    /// automatically be the end of the previous edge.
    Continuous,
}

pub fn create_edges_setup(
    In(key): In<BufferKey<CreateEdges>>,
    mut access: BufferAccessMut<CreateEdges>,
    cursor: Res<Cursor>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    if state.preview_edge.is_none() {
        state.initialize_preview(cursor.level_anchor_placement, &mut commands);
    }
    Ok(())
}

pub fn on_hover_for_create_edges(
    In((hover, key)): In<(Hover, BufferKey<CreateEdges>)>,
    mut access: BufferAccessMut<CreateEdges>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut edges: Query<&mut Edge<Entity>>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    // TODO(@mxgrey): Consider moving this logic into AnchorFilter since it gets
    // used by all the different anchor selection modes.
    let anchor = match hover.0 {
        Some(anchor) => {
            cursor.remove_mode(SELECT_ANCHOR_MODE_LABEL, &mut visibility);
            anchor
        }
        None => {
            cursor.add_mode(SELECT_ANCHOR_MODE_LABEL, &mut visibility);
            cursor.level_anchor_placement
        }
    };

    if let Some(preview) = &mut state.preview_edge {
        // If we already have an active preview, then use the new anchor for the
        // side that we currently need to select for.
        let index = preview.side.index();
        let mut edge = edges.get_mut(preview.edge).or_broken_query()?;

        let old_anchor = edge.array()[index];
        if old_anchor != anchor {
            let opposite_anchor = edge.array()[preview.side.opposite().index()];
            if opposite_anchor != old_anchor {
                commands.queue(ChangeDependent::remove(old_anchor, preview.edge));
            }

            edge.array_mut()[index] = anchor;
            commands.queue(ChangeDependent::add(anchor, preview.edge));
        }
    } else {
        // There is currently no active preview, so we need to create one.
        let edge = Edge::new(anchor, anchor);
        let edge = (state.spawn_edge)(edge, &mut commands);
        state.preview_edge = Some(PreviewEdge {
            edge,
            side: Side::start(),
            provisional_start: false,
        });
        commands.queue(ChangeDependent::add(anchor, edge));
    }

    Ok(())
}

pub fn on_select_for_create_edges(
    In((selection, key)): In<(SelectionCandidate, BufferKey<CreateEdges>)>,
    mut access: BufferAccessMut<CreateEdges>,
    mut edges: Query<&mut Edge<Entity>>,
    mut commands: Commands,
    cursor: Res<Cursor>,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    let anchor = selection.candidate;
    if let Some(preview) = &mut state.preview_edge {
        match preview.side {
            Side::Left => {
                // We are pinning down the first anchor of the edge
                let mut edge = edges.get_mut(preview.edge).or_broken_query()?;
                commands.queue(ChangeDependent::remove(edge.left(), preview.edge));
                *edge.left_mut() = anchor;
                commands.queue(ChangeDependent::add(anchor, preview.edge));

                if edge.right() != anchor {
                    commands.queue(ChangeDependent::remove(edge.right(), preview.edge));
                }

                *edge.right_mut() = cursor.level_anchor_placement;
                commands.queue(ChangeDependent::add(
                    cursor.level_anchor_placement,
                    preview.edge,
                ));

                preview.side = Side::Right;
                preview.provisional_start = selection.provisional;
            }
            Side::Right => {
                // We are finishing the edge
                let mut edge = edges.get_mut(preview.edge).or_broken_query()?;
                if edge.left() == anchor {
                    // The user is trying to use the same point for the start
                    // and end of an edge. Issue a warning and exit early.
                    warn!(
                        "You are trying to select an anchor {:?} for both the \
                        start and end points of an edge, which is not allowed.",
                        anchor,
                    );
                    return Ok(());
                }
                *edge.right_mut() = anchor;
                commands.queue(ChangeDependent::add(anchor, preview.edge));
                commands
                    .get_entity(preview.edge)
                    .or_broken_query()?
                    .remove::<Pending>();

                match state.continuity {
                    EdgeContinuity::Single => {
                        state.preview_edge = None;
                        // This simply means we are terminating the workflow now
                        // because we have finished drawing the single edge
                        return Err(None);
                    }
                    EdgeContinuity::Separate => {
                        // Start drawing a new edge from a blank slate with the
                        // next selection
                        state.initialize_preview(cursor.level_anchor_placement, &mut commands);
                    }
                    EdgeContinuity::Continuous => {
                        // Start drawing a new edge, picking up from the end
                        // point of the previous edge
                        let edge = Edge::new(anchor, cursor.level_anchor_placement);
                        let edge = (state.spawn_edge)(edge, &mut commands);
                        state.preview_edge = Some(PreviewEdge {
                            edge,
                            side: Side::end(),
                            provisional_start: false,
                        });
                        commands.queue(ChangeDependent::add(anchor, edge));
                        commands.queue(ChangeDependent::add(cursor.level_anchor_placement, edge));
                    }
                }
            }
        }
    } else {
        // We have no preview at all yet somehow, so we'll need to create a
        // fresh new edge to insert the selected anchor into
        let edge = Edge::new(anchor, anchor);
        let edge = (state.spawn_edge)(edge, &mut commands);
        state.preview_edge = Some(PreviewEdge {
            edge,
            side: Side::start(),
            provisional_start: selection.provisional,
        });
    }

    Ok(())
}

pub fn on_keyboard_for_create_edges(
    In((button, key)): In<(KeyCode, BufferKey<CreateEdges>)>,
    mut access: BufferAccessMut<CreateEdges>,
    mut edges: Query<&'static mut Edge<Entity>>,
    cursor: Res<Cursor>,
    mut commands: Commands,
) -> SelectionNodeResult {
    if !matches!(button, KeyCode::Escape) {
        // The button was not the escape key, so there's nothing for us to do
        // here.
        return Ok(());
    }

    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    if let Some(preview) = &mut state.preview_edge {
        if preview.side == Side::end() {
            // We currently have an active preview edge and are selecting for
            // the second point in the edge. Esc means we should back out of the
            // current edge without exiting the edge creation workflow so the
            // user can choose a different start point.
            let mut edge = edges.get_mut(preview.edge).or_broken_query()?;
            for anchor in edge.array() {
                commands.queue(ChangeDependent::remove(anchor, preview.edge));
            }
            if preview.provisional_start {
                commands
                    .get_entity(edge.start())
                    .or_broken_query()?
                    .despawn_recursive();
            }

            *edge.left_mut() = cursor.level_anchor_placement;
            *edge.right_mut() = cursor.level_anchor_placement;
            preview.side = Side::start();
            preview.provisional_start = false;
            commands.queue(ChangeDependent::add(
                cursor.level_anchor_placement,
                preview.edge,
            ));
        } else {
            // We are selecting for the first point in the edge. If the user has
            // pressed Esc then that means they want to stop creating edges
            // altogether. Return Err(None) to indicate that the workflow should
            // exit cleaning.
            return Err(None);
        }
    } else {
        // We currently have no preview active at all. If the user hits Esc then
        // they want to exit the workflow altogether.
        return Err(None);
    }

    Ok(())
}

pub fn cleanup_create_edges(
    In(key): In<BufferKey<CreateEdges>>,
    mut access: BufferAccessMut<CreateEdges>,
    edges: Query<&'static Edge<Entity>>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.pull().or_broken_state()?;

    if let Some(preview) = state.preview_edge {
        // We created a preview, so we should despawn it while cleaning up
        preview.cleanup(&edges, &mut commands)?;
    }
    Ok(())
}
