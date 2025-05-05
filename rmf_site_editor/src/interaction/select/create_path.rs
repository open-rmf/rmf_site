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
use rmf_site_format::Path;
use std::borrow::Borrow;

use std::collections::HashSet;

pub fn spawn_create_path_service(
    helpers: &AnchorSelectionHelpers,
    app: &mut App,
) -> Service<Option<Entity>, ()> {
    let anchor_setup =
        app.spawn_service(anchor_selection_setup::<CreatePath>.into_blocking_service());
    let state_setup = app.spawn_service(create_path_setup.into_blocking_service());
    let update_preview = app.spawn_service(on_hover_for_create_path.into_blocking_service());
    let update_current = app.spawn_service(on_select_for_create_path.into_blocking_service());
    let handle_key_code = app.spawn_service(exit_on_esc::<CreatePath>.into_blocking_service());
    let cleanup_state = app.spawn_service(cleanup_create_path.into_blocking_service());

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

pub struct CreatePath {
    /// Function pointer for spawning an initial path.
    pub spawn_path: fn(Path<Entity>, &mut Commands) -> Entity,
    /// The path which is being built. This will initially be [`None`] until setup
    /// happens, then `spawn_path` will be used to create this. For all the
    /// services in the `create_path` workflow besides setup, this should
    /// contain [`Some`].
    ///
    /// If points are being added to an existing path, this could be initialized
    /// as [`Some`] before the state is passed into the workflow.
    pub path: Option<Entity>,
    /// A minimum for how many points need to be selected for the path to be
    /// considered valid. Use 0 if there is no minimum.
    pub minimum_points: usize,
    /// Whether the path is allowed to have an inner loop. E.g.
    /// `A -> B -> C -> D -> B` would be an inner loop.
    pub allow_inner_loops: bool,
    /// The path is implied to always be a complete loop. This has two consequences:
    /// 1. If the first point gets re-selected later in the path then we automatically
    ///    consider the path to be finished.
    /// 2. When (1) occurs, the first point does not get re-added to the path.
    pub implied_complete_loop: bool,
    /// A list of all anchors being used in the path which are provisional,
    /// meaning they should be despawned if the path creation ends before
    /// reaching the minimum number of points.
    pub provisional_anchors: HashSet<Entity>,
    pub scope: AnchorScope,
}

impl CreatePath {
    pub fn new(
        spawn_path: fn(Path<Entity>, &mut Commands) -> Entity,
        minimum_points: usize,
        allow_inner_loops: bool,
        implied_complete_loop: bool,
        scope: AnchorScope,
    ) -> Self {
        Self {
            spawn_path,
            path: None,
            allow_inner_loops,
            minimum_points,
            implied_complete_loop,
            scope,
            provisional_anchors: Default::default(),
        }
    }

    pub fn set_last(
        &self,
        chosen: Entity,
        path_mut: &mut Path<Entity>,
        commands: &mut Commands,
    ) -> SelectionNodeResult {
        let path = self.path.or_broken_state()?;
        let last = path_mut.0.last_mut().or_broken_state()?;
        if chosen == *last {
            // Nothing to change
            return Ok(());
        }

        let previous = *last;
        *last = chosen;
        if !path_mut.0.contains(&previous) {
            commands.queue(ChangeDependent::remove(previous, path));
        }

        commands.queue(ChangeDependent::add(chosen, path));
        Ok(())
    }
}

impl Borrow<AnchorScope> for CreatePath {
    fn borrow(&self) -> &AnchorScope {
        &self.scope
    }
}

pub fn create_path_with_texture<T: Bundle + From<Path<Entity>>>(
    path: Path<Entity>,
    commands: &mut Commands,
) -> Entity {
    let new_bundle: T = path.into();
    commands
        .spawn((new_bundle, TextureNeedsAssignment, Pending))
        .id()
}

pub fn create_path_setup(
    In(key): In<BufferKey<CreatePath>>,
    mut access: BufferAccessMut<CreatePath>,
    cursor: Res<Cursor>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    if state.path.is_none() {
        let path = Path(vec![cursor.level_anchor_placement]);
        let path = (state.spawn_path)(path, &mut commands);
        commands.queue(ChangeDependent::add(cursor.level_anchor_placement, path));
        state.path = Some(path);
    }

    Ok(())
}

pub fn on_hover_for_create_path(
    In((hover, key)): In<(Hover, BufferKey<CreatePath>)>,
    mut access: BufferAccessMut<CreatePath>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut paths: Query<&mut Path<Entity>>,
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

    let path = state.path.or_broken_state()?;
    let mut path_mut = paths.get_mut(path).or_broken_query()?;
    state.set_last(chosen, path_mut.as_mut(), &mut commands)
}

pub fn on_select_for_create_path(
    In((selection, key)): In<(SelectionCandidate, BufferKey<CreatePath>)>,
    mut access: BufferAccessMut<CreatePath>,
    mut paths: Query<&mut Path<Entity>>,
    mut commands: Commands,
    cursor: Res<Cursor>,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    let chosen = selection.candidate;
    let provisional = selection.provisional;
    let path = state.path.or_broken_state()?;
    let mut path_mut = paths.get_mut(path).or_broken_query()?;

    if state.implied_complete_loop {
        let first = path_mut.0.first().or_broken_state()?;
        if chosen == *first && path_mut.0.len() >= state.minimum_points {
            // The user has re-selected the first point and there are enough
            // points in the path to meet the minimum requirement, so we can
            // just end the workflow.
            return Err(None);
        }
    }

    if !state.allow_inner_loops {
        for a in &path_mut.0[..path_mut.0.len() - 1] {
            if *a == chosen {
                warn!(
                    "Attempting to create an inner loop in a type of path \
                    which does not allow inner loops."
                );
                return Ok(());
            }
        }
    }

    if path_mut.0.len() >= 2 {
        if let Some(second_to_last) = path_mut.0.get(path_mut.0.len() - 2) {
            if *second_to_last == chosen {
                // Even if inner loops are allowed, we should never allow the same
                // anchor to be chosen twice in a row.
                warn!("Trying to select the same anchor for a path twice in a row");
                return Ok(());
            }
        }
    }

    state.set_last(chosen, path_mut.as_mut(), &mut commands)?;
    if provisional {
        state.provisional_anchors.insert(chosen);
    }

    path_mut.0.push(cursor.level_anchor_placement);
    commands.queue(ChangeDependent::add(cursor.level_anchor_placement, path));

    Ok(())
}

pub fn cleanup_create_path(
    In(key): In<BufferKey<CreatePath>>,
    mut access: BufferAccessMut<CreatePath>,
    mut paths: Query<&'static mut Path<Entity>>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.pull().or_broken_state()?;

    let Some(path) = state.path else {
        // If there is no path then there is nothing to cleanup. This might
        // happen if the setup needed to bail out for some reason.
        return Ok(());
    };
    commands
        .get_entity(path)
        .or_broken_query()?
        .remove::<Pending>();
    let mut path_mut = paths.get_mut(path).or_broken_query()?;

    // First check if the len-1 meets the minimum point requirement. If not we
    // should despawn the path as well as any provisional anchors that it used.
    if path_mut.0.len() - 1 < state.minimum_points {
        // We did not collect enough points for the path so we should despawn it
        // as well as any provisional points it contains.
        for a in &path_mut.0 {
            commands.queue(ChangeDependent::remove(*a, path));
        }

        for a in state.provisional_anchors {
            if let Some(a_mut) = commands.get_entity(a) {
                a_mut.despawn_recursive();
            }
        }

        commands
            .get_entity(path)
            .or_broken_query()?
            .despawn_recursive();
    } else {
        if let Some(a) = path_mut.0.last() {
            // The last point in the path is always a preview point so we need
            // to pop it.
            let a = *a;
            path_mut.0.pop();
            if !path_mut.contains(&a) {
                // Remove the dependency on the last point since it no longer
                // exists in the path
                commands.queue(ChangeDependent::remove(a, path));
            }
        }

        if path_mut.0.is_empty() {
            // The path is empty... we shouldn't keep an empty path so let's
            // just despawn it.
            commands
                .get_entity(path)
                .or_broken_query()?
                .despawn_recursive();
        }
    }

    Ok(())
}
