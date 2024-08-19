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
    site::{ChangeDependent, Pending},
};
use rmf_site_format::Point;
use bevy::prelude::*;
use bevy_impulse::*;
use std::borrow::Borrow;

pub fn spawn_create_point_service(
    helpers: &AnchorSelectionHelpers,
    app: &mut App,
) -> Service<Option<Entity>, ()> {
    let anchor_setup = app.spawn_service(anchor_selection_setup::<CreatePoint>.into_blocking_service());
    let state_setup = app.spawn_service(create_point_setup.into_blocking_service());
    let update_preview = app.spawn_service(on_hover_for_create_point.into_blocking_service());
    let update_current = app.spawn_service(on_select_for_create_point.into_blocking_service());
    let handle_key_code = app.spawn_service(exit_on_esc::<CreatePoint>.into_blocking_service());
    let cleanup_state = app.spawn_service(cleanup_create_point.into_blocking_service());

    helpers.spawn_anchor_selection_workflow(
        anchor_setup,
        state_setup,
        update_preview,
        update_current,
        handle_key_code,
        cleanup_state,
        &mut app.world,
    )
}

pub struct CreatePoint {
    /// Function pointer for spawning a point.
    pub spawn_point: fn(Point<Entity>, &mut Commands) -> Entity,
    /// The point which is being created. This will initially be [`None`] until
    /// setup happens, then `spawn_point` will be used to create this. For all
    /// the services in the `create_point` workflow besides setup, this should
    /// contain [`Some`].
    pub point: Option<Entity>,
    /// True if we should keep creating new points until the user presses Esc,
    /// False if we should only create one point.
    pub repeating: bool,
    pub scope: AnchorScope,
}

impl CreatePoint {
    pub fn new<T: Bundle + From<Point<Entity>>>(
        repeating: bool,
        scope: AnchorScope,
    ) -> Self {
        Self {
            spawn_point: create_point::<T>,
            point: None,
            repeating,
            scope,
        }
    }

    pub fn create_new_point(
        &mut self,
        anchor: Entity,
        commands: &mut Commands,
    ) {
        let point = Point(anchor);
        let point = (self.spawn_point)(point, commands);
        commands.add(ChangeDependent::add(anchor, point));
        self.point = Some(point);
    }
}

impl Borrow<AnchorScope> for CreatePoint {
    fn borrow(&self) -> &AnchorScope {
        &self.scope
    }
}

fn create_point<T: Bundle + From<Point<Entity>>>(
    point: Point<Entity>,
    commands: &mut Commands,
) -> Entity {
    let new_bundle: T = point.into();
    commands.spawn((new_bundle, Pending)).id()
}

pub fn create_point_setup(
    In(key): In<BufferKey<CreatePoint>>,
    mut access: BufferAccessMut<CreatePoint>,
    cursor: Res<Cursor>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    if state.point.is_none() {
        state.create_new_point(cursor.level_anchor_placement, &mut commands);
    }

    Ok(())
}

fn change_point(
    chosen: Entity,
    point: Entity,
    points: &mut Query<&mut Point<Entity>>,
    commands: &mut Commands,
) -> SelectionNodeResult {
    let mut point_mut = points.get_mut(point).or_broken_query()?;
    if point_mut.0 == chosen {
        return Ok(());
    }

    commands.add(ChangeDependent::remove(point_mut.0, point));
    commands.add(ChangeDependent::add(chosen, point));
    point_mut.0 = chosen;
    Ok(())
}

pub fn on_hover_for_create_point(
    In((hover, key)): In<(Hover, BufferKey<CreatePoint>)>,
    mut access: BufferAccessMut<CreatePoint>,
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

    let point = state.point.or_broken_state()?;
    change_point(chosen, point, &mut points, &mut commands)
}

pub fn on_select_for_create_point(
    In((selection, key)): In<(SelectionCandidate, BufferKey<CreatePoint>)>,
    mut access: BufferAccessMut<CreatePoint>,
    cursor: Res<Cursor>,
    mut points: Query<&mut Point<Entity>>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;
    let point = state.point.or_broken_state()?;
    change_point(selection.candidate, point, &mut points, &mut commands)?;
    commands.get_entity(point).or_broken_query()?.remove::<Pending>();
    if state.repeating {
        state.create_new_point(cursor.level_anchor_placement, &mut commands);
        return Ok(());
    } else {
        state.point = None;
        return Err(None);
    }
}

pub fn cleanup_create_point(
    In(key): In<BufferKey<CreatePoint>>,
    mut access: BufferAccessMut<CreatePoint>,
    points: Query<&'static Point<Entity>>,
    mut commands: Commands,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.pull().or_broken_state()?;

    let Some(point) = state.point else {
        // If there is no point then there is nothing to cleanup.
        return Ok(());
    };

    let point_ref = points.get(point).or_broken_query()?;
    commands.add(ChangeDependent::remove(point_ref.0, point));
    commands.get_entity(point).or_broken_query()?.despawn_recursive();

    Ok(())
}
