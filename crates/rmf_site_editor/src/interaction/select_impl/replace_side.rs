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
    site::{ChangeDependent, Original},
};
use anyhow::Error as Anyhow;
use bevy::prelude::*;
use crossflow::*;
use rmf_site_format::{Edge, Side};
use std::borrow::Borrow;

pub fn spawn_replace_side_service(
    helpers: &AnchorSelectionHelpers,
    app: &mut App,
) -> Service<Option<Entity>, ()> {
    let anchor_setup =
        app.spawn_service(anchor_selection_setup::<ReplaceSide>.into_blocking_service());
    let state_setup = app.spawn_service(replace_side_setup);
    let update_preview = app.spawn_service(on_hover_for_replace_side.into_blocking_service());
    let update_current = app.spawn_service(on_select_for_replace_side.into_blocking_service());
    let handle_key_code = app.spawn_service(exit_on_esc::<ReplaceSide>.into_blocking_service());
    let cleanup_state = app.spawn_service(cleanup_replace_side.into_blocking_service());

    helpers.spawn_anchor_selection_workflow(
        anchor_setup,
        state_setup,
        update_preview,
        update_current.optional_stream_cast(),
        handle_key_code,
        cleanup_state,
        app.world_mut(),
    )
}

pub struct ReplaceSide {
    /// The edge whose anchor is being replaced
    pub edge: Entity,
    /// The side of the edge which is being replaced
    pub side: Side,
    /// The original values for the edge. This is None until setup occurs, then
    /// its value will be available.
    pub original: Option<Edge<Entity>>,
    /// The scope that the edge exists in
    pub scope: AnchorScope,
    /// Keeps track of whether the replacement really happened. If false, the
    /// cleanup will revert the edge to its original state. If true, the cleanup
    /// will not need to do anything.
    pub replaced: bool,
    /// Whether or not the replaced side must be on a consistent level with the original.
    pub level_consistency: bool,
}

impl ReplaceSide {
    pub fn new(edge: Entity, side: Side, scope: AnchorScope) -> Self {
        Self {
            edge,
            side,
            scope,
            original: None,
            replaced: false,
            level_consistency: true,
        }
    }

    pub fn set_chosen(
        &mut self,
        chosen: Entity,
        edges: &mut Query<&mut Edge<Entity>>,
        parents: &Query<&ChildOf>,
        lifts: &Query<(), With<LiftCabin<Entity>>>,
        cursor_anchor: Entity,
        commands: &mut Commands,
    ) -> Result<bool, Option<Anyhow>> {
        let original = self.original.or_broken_buffer()?;
        if self.level_consistency && chosen != cursor_anchor {
            let a = original.array()[self.side.index()];
            if !are_anchors_siblings(a, chosen, &parents, &lifts)? {
                warn!("Unable to use selected anchor because it is on an incompatible level");
                return Ok(false);
            }
        }

        let mut edge_mut = edges.get_mut(self.edge).or_broken_query()?;

        for a in edge_mut.array() {
            // Remove both current dependencies in case both of them change.
            // If either dependency doesn't change then they'll be added back
            // later anyway.
            commands.queue(ChangeDependent::remove(a, self.edge));
        }

        if chosen == original.array()[self.side.opposite().index()] {
            // The user is choosing the anchor on the opposite side of the edge as
            // the replacement anchor. We take this to mean that the user wants to
            // flip the edge.
            *edge_mut.left_mut() = original.right();
            *edge_mut.right_mut() = original.left();
        } else {
            edge_mut.array_mut()[self.side.index()] = chosen;
            let opp = self.side.opposite().index();
            edge_mut.array_mut()[opp] = original.array()[opp];
        }

        for a in edge_mut.array() {
            commands.queue(ChangeDependent::add(a, self.edge));
        }

        Ok(true)
    }
}

impl Borrow<AnchorScope> for ReplaceSide {
    fn borrow(&self) -> &AnchorScope {
        &self.scope
    }
}

type SetupService = BlockingServiceInput<BufferKey<ReplaceSide>, StreamOf<SelectionAlignmentBasis>>;

pub fn replace_side_setup(
    In(BlockingService {
        request: key,
        streams,
        ..
    }): SetupService,
    mut access: BufferAccessMut<ReplaceSide>,
    mut edges: Query<&mut Edge<Entity>>,
    parents: Query<&ChildOf>,
    lifts: Query<(), With<LiftCabin<Entity>>>,
    cursor: Res<Cursor>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    let edge_ref = edges.get(state.edge).or_broken_query()?;
    let original_edge: Edge<Entity> = *edge_ref;
    state.original = Some(original_edge);
    commands.entity(state.edge).insert(Original(original_edge));
    state.set_chosen(
        cursor.level_anchor_placement,
        &mut edges,
        &parents,
        &lifts,
        cursor.level_anchor_placement,
        &mut commands,
    )?;

    // Set the anchor on the opposite side of the edge as the alignment basis.
    streams.send(SelectionAlignmentBasis::new(
        original_edge.array()[state.side.opposite().index()],
    ));

    Ok(())
}

pub fn on_hover_for_replace_side(
    In((hover, key)): In<(Hover, BufferKey<ReplaceSide>)>,
    mut access: BufferAccessMut<ReplaceSide>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut edges: Query<&mut Edge<Entity>>,
    parents: Query<&ChildOf>,
    lifts: Query<(), With<LiftCabin<Entity>>>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    let chosen = match hover.0 {
        Some(anchor) => {
            cursor.remove_mode(SELECT_ANCHOR_MODE_LABEL, &mut visibility);
            anchor
        }
        None => {
            cursor.add_mode(SELECT_ANCHOR_MODE_LABEL, &mut visibility);
            cursor.level_anchor_placement
        }
    };

    state.set_chosen(
        chosen,
        &mut edges,
        &parents,
        &lifts,
        cursor.level_anchor_placement,
        &mut commands,
    )?;

    Ok(())
}

pub fn on_select_for_replace_side(
    In((selection, key)): In<(SelectionCandidate, BufferKey<ReplaceSide>)>,
    mut access: BufferAccessMut<ReplaceSide>,
    mut edges: Query<&mut Edge<Entity>>,
    parents: Query<&ChildOf>,
    lifts: Query<(), With<LiftCabin<Entity>>>,
    cursor: Res<Cursor>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;
    if state.set_chosen(
        selection.candidate,
        &mut edges,
        &parents,
        &lifts,
        cursor.level_anchor_placement,
        &mut commands,
    )? {
        state.replaced = true;
        // Since the selection has been made, we should exit the workflow now
        return Err(None);
    }

    Ok(())
}

pub fn cleanup_replace_side(
    In(key): In<BufferKey<ReplaceSide>>,
    mut access: BufferAccessMut<ReplaceSide>,
    mut edges: Query<&'static mut Edge<Entity>>,
    parents: Query<&ChildOf>,
    lifts: Query<(), With<LiftCabin<Entity>>>,
    cursor: Res<Cursor>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let mut state = access.pull().or_broken_state()?;

    commands
        .get_entity(state.edge)
        .or_broken_query()?
        .remove::<Original<Edge<Entity>>>();

    if state.replaced {
        // The anchor was fully replaced, so nothing further to do
        return Ok(());
    }

    // The anchor was not replaced so we need to revert to the original setup
    let Some(original) = state.original else {
        return Ok(());
    };

    let revert = original.array()[state.side.index()];
    state.set_chosen(
        revert,
        &mut edges,
        &parents,
        &lifts,
        cursor.level_anchor_placement,
        &mut commands,
    )?;

    Ok(())
}
