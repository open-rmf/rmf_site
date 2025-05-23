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
use bevy::prelude::*;
use bevy_impulse::*;
use rmf_site_format::Point;
use std::borrow::Borrow;

pub fn spawn_replace_point_service(
    helpers: &AnchorSelectionHelpers,
    app: &mut App,
) -> Service<Option<Entity>, ()> {
    let anchor_setup =
        app.spawn_service(anchor_selection_setup::<ReplacePoint>.into_blocking_service());
    let state_setup = app.spawn_service(replace_point_setup.into_blocking_service());
    let update_preview = app.spawn_service(on_hover_for_replace_point.into_blocking_service());
    let update_current = app.spawn_service(on_select_for_replace_point.into_blocking_service());
    let handle_key_code = app.spawn_service(exit_on_esc::<ReplacePoint>.into_blocking_service());
    let cleanup_state = app.spawn_service(cleanup_replace_point.into_blocking_service());

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

pub struct ReplacePoint {
    /// The point whose anchor is being replaced
    pub point: Entity,
    /// The original value of the point. This is None until setup occurs, then
    /// its value will be available.
    pub original: Option<Point<Entity>>,
    /// The scope that the point exists in
    pub scope: AnchorScope,
    /// Keeps track of whether the replacement really happened. If false, the
    /// cleanup will revert the point to its original state. If true, the cleanup
    /// will not need to do anything.
    pub replaced: bool,
}

impl ReplacePoint {
    pub fn new(point: Entity, scope: AnchorScope) -> Self {
        Self {
            point,
            original: None,
            scope,
            replaced: false,
        }
    }

    pub fn set_chosen(
        &mut self,
        chosen: Entity,
        points: &mut Query<&mut Point<Entity>>,
        commands: &mut Commands,
    ) -> SelectionNodeResult {
        let mut point_mut = points.get_mut(self.point).or_broken_query()?;
        commands.queue(ChangeDependent::remove(point_mut.0, self.point));
        point_mut.0 = chosen;
        commands.queue(ChangeDependent::add(chosen, self.point));
        Ok(())
    }
}

impl Borrow<AnchorScope> for ReplacePoint {
    fn borrow(&self) -> &AnchorScope {
        &self.scope
    }
}

pub fn replace_point_setup(
    In(key): In<BufferKey<ReplacePoint>>,
    mut access: BufferAccessMut<ReplacePoint>,
    mut points: Query<&'static mut Point<Entity>>,
    cursor: Res<Cursor>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    let original = *points.get(state.point).or_broken_query()?;
    state.original = Some(original);
    commands.entity(state.point).insert(Original(original));
    state.set_chosen(cursor.level_anchor_placement, &mut points, &mut commands)?;

    Ok(())
}

pub fn on_hover_for_replace_point(
    In((hover, key)): In<(Hover, BufferKey<ReplacePoint>)>,
    mut access: BufferAccessMut<ReplacePoint>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut points: Query<&mut Point<Entity>>,
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

    state.set_chosen(chosen, &mut points, &mut commands)
}

pub fn on_select_for_replace_point(
    In((selection, key)): In<(SelectionCandidate, BufferKey<ReplacePoint>)>,
    mut access: BufferAccessMut<ReplacePoint>,
    mut points: Query<&mut Point<Entity>>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;
    state.set_chosen(selection.candidate, &mut points, &mut commands)?;
    state.replaced = true;
    // Since the selection has been made, we should exit the workflow now
    Err(None)
}

pub fn cleanup_replace_point(
    In(key): In<BufferKey<ReplacePoint>>,
    mut access: BufferAccessMut<ReplacePoint>,
    mut points: Query<&'static mut Point<Entity>>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let mut state = access.pull().or_broken_state()?;

    commands
        .get_entity(state.point)
        .or_broken_query()?
        .remove::<Original<Point<Entity>>>();

    if state.replaced {
        // The anchor was fully replaced, so nothing furtehr to do
        return Ok(());
    }

    let Some(original) = state.original else {
        return Ok(());
    };

    state.set_chosen(original.0, &mut points, &mut commands)?;

    Ok(())
}
