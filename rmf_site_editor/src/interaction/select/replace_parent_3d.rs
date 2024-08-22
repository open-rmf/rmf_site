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
    interaction::select::*,
    site::{Dependents, FrameMarker},
    widgets::canvas_tooltips::CanvasTooltips,
};
use bevy::prelude::Input as UserInput;
use bevy_mod_raycast::deferred::RaycastSource;
use std::borrow::Cow;

pub fn spawn_replace_parent_3d_workflow(
    hover_service: Service<(), (), Hover>,
    app: &mut App,
) -> Service<Option<Entity>, ()> {
    let setup = app.spawn_service(replace_parent_3d_setup.into_blocking_service());
    let find_parent = app.spawn_continuous_service(Update, replace_parent_3d_find_parent);
    let parent_chosen = app.spawn_service(replace_parent_3d_parent_chosen.into_blocking_service());
    let handle_key_code = app.spawn_service(on_keyboard_for_replace_parent_3d.into_blocking_service());
    let cleanup = app.spawn_service(replace_parent_3d_cleanup.into_blocking_service());
    let keyboard_just_pressed = app
        .world
        .resource::<KeyboardServices>()
        .keyboard_just_pressed;

    app.world.spawn_io_workflow(move |scope, builder| {
        let buffer = builder.create_buffer::<ReplaceParent3d>(BufferSettings::keep_last(1));
        let initial_state = scope
            .input
            .chain(builder)
            .then(extract_selector_input::<ReplaceParent3d>.into_blocking_callback())
            .branch_for_err(|err| err.connect(scope.terminate))
            .cancel_on_none()
            .output()
            .fork_clone(builder);

        let begin_input_services = initial_state
            .clone_chain(builder)
            .then_push(buffer)
            .then_access(buffer)
            .then(setup)
            .branch_for_err(|err| err.map_block(print_if_err).connect(scope.terminate))
            .output()
            .fork_clone(builder);

        begin_input_services
            .clone_chain(builder)
            .then_node(keyboard_just_pressed)
            .streams
            .chain(builder)
            .inner()
            .then(handle_key_code)
            .dispose_on_ok()
            .map_block(print_if_err)
            .connect(scope.terminate);

        begin_input_services
            .clone_chain(builder)
            .then(hover_service)
            .unused();

        (
            initial_state.clone_output(builder),
            begin_input_services.clone_output(builder),
        )
            .join(builder)
            .map_block(|(state, _)| state.object)
            .then(find_parent)
            .with_access(buffer)
            .then(parent_chosen)
            .branch_for_err(|err| err.map_block(print_if_err).connect(scope.terminate))
            .connect(scope.terminate);

        builder.on_cleanup(buffer, move |scope, builder| {
            scope.input.chain(builder).then(cleanup).fork_result(
                |ok| ok.connect(scope.terminate),
                |err| err.map_block(print_if_err).connect(scope.terminate),
            );
        });
    })
}

#[derive(Clone, Copy, Debug)]
pub struct ReplaceParent3d {
    pub object: Entity,
    pub workspace: Entity,
}

pub fn replace_parent_3d_setup(
    In(key): In<BufferKey<ReplaceParent3d>>,
    mut access: BufferAccessMut<ReplaceParent3d>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut highlight: ResMut<HighlightAnchors>,
    mut gizmo_blockers: ResMut<GizmoBlockers>,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_buffer()?;

    highlight.0 = true;
    gizmo_blockers.selecting = true;

    // We use the workspace entity as a blocker because it's a unique ID that
    // will be consistent and available during cleanup and which will not be
    // toggled as a blocker by other system in the application. This is to make
    // sure that the cursor frame does not turn on during a change in
    // highlighting.
    cursor.add_blocker(state.workspace, &mut visibility);
    Ok(())
}

pub fn replace_parent_3d_cleanup(
    In(key): In<BufferKey<ReplaceParent3d>>,
    mut access: BufferAccessMut<ReplaceParent3d>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut highlight: ResMut<HighlightAnchors>,
    mut gizmo_blockers: ResMut<GizmoBlockers>,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_buffer()?;

    highlight.0 = false;
    gizmo_blockers.selecting = false;
    cursor.remove_blocker(state.workspace, &mut visibility);
    Ok(())
}

pub fn replace_parent_3d_find_parent(
    In(ContinuousService { key }): ContinuousServiceInput<Entity, Option<Entity>>,
    mut orders: ContinuousQuery<Entity, Option<Entity>>,
    mut tooltips: ResMut<CanvasTooltips>,
    mut hover: EventWriter<Hover>,
    raycast_sources: Query<&RaycastSource<SiteRaycastSet>>,
    parents: Query<&Parent>,
    mut filter: PlaceObject3dFilter,
    hovering: Res<Hovering>,
    mouse_button_input: Res<UserInput<MouseButton>>,
    blockers: Option<Res<PickingBlockers>>,
    mut selected: EventReader<Select>,
) {
    let Some(mut orders) = orders.get_mut(&key) else {
        selected.clear();
        return;
    };

    let Some(order) = orders.get_mut(0) else {
        // Clear the selected reader so we don't mistake an earlier signal as
        // being intended for this workflow.
        selected.clear();
        return;
    };

    tooltips.add(Cow::Borrowed("Select new parent"));

    let object = *order.request();
    for s in selected.read() {
        // Allow users to signal the choice of parent by means other than clicking
        match s.0 {
            Some(s) => {
                if let Some(e) = filter.filter_pick(s.candidate) {
                    order.respond(Some(e));
                    return;
                }

                info!("Received parent replacement selection signal for an invalid parent candidate");
            }
            None => {
                // The user has sent a signal to remove the object from its parent
                order.respond(None);
                return;
            }
        }
    }

    let Ok(source) = raycast_sources.get_single() else {
        return;
    };

    let mut hovered: Option<Entity> = None;
    let mut ignore_click = false;
    for (e, _) in source.intersections() {
        let Some(e) = filter.filter_pick(*e) else {
            continue;
        };

        if AncestorIter::new(&parents, e).filter(|e| *e == object).next().is_some() {
            ignore_click = true;
            tooltips.add(Cow::Borrowed("Cannot select a child of the object to be its parent"));
            break;
        }

        if e == object {
            ignore_click = true;
            tooltips.add(Cow::Borrowed("Cannot select an object to be its own parent"));
            break;
        }

        hovered = Some(e);
    }

    if hovered != hovering.0 {
        hover.send(Hover(hovered));
    }

    let clicked = mouse_button_input.just_pressed(MouseButton::Left);
    let blocked = blockers.filter(|x| x.blocking()).is_some();
    if clicked && !blocked && !ignore_click {
        order.respond(hovered);
    }
}

pub fn replace_parent_3d_parent_chosen(
    In((parent, key)): In<(Option<Entity>, BufferKey<ReplaceParent3d>)>,
    access: BufferAccess<ReplaceParent3d>,
    mut dependents: Query<&mut Dependents>,
    mut poses: Query<&mut Pose>,
    global_tfs: Query<&GlobalTransform>,
    parents: Query<&Parent>,
    frames: Query<(), With<FrameMarker>>,
    mut commands: Commands,
    mut anchors: Query<&mut Anchor>,
) -> SelectionNodeResult {
    let access = access.get(&key).or_broken_buffer()?;
    let state = access.newest().or_broken_state()?;

    let parent = parent.and_then(|p| {
        if frames.contains(p) {
            Some(p)
        } else {
            // The selected parent is not a frame, so find its first ancestor
            // that contains a FrameMarker
            AncestorIter::new(&parents, p).find(|e| frames.contains(*e))
        }
    })
    .unwrap_or(state.workspace);

    let previous_parent = parents.get(state.object).or_broken_query()?.get();
    if parent == previous_parent {
        info!("Object's parent remains the same");
        return Ok(());
    }

    let object_tf = global_tfs.get(state.object).or_broken_query()?.affine();
    let inv_parent_tf = global_tfs.get(parent).or_broken_query()?.affine().inverse();
    let relative_pose: Pose = Transform::from_matrix((inv_parent_tf * object_tf).into()).into();

    let [mut previous_deps, mut new_deps] = dependents.get_many_mut(
        [previous_parent, parent]
    ).or_broken_query()?;

    if let Ok(mut pose_mut) = poses.get_mut(state.object) {
        *pose_mut = relative_pose;
    } else {
        let mut anchor = anchors.get_mut(state.object).or_broken_query()?;
        *anchor = Anchor::Pose3D(relative_pose);
    }

    // Do all mutations after everything is successfully queried so we don't
    // risk an inconsistent/broken world due to a query failing.
    commands.get_entity(state.object).or_broken_query()?.set_parent(parent);
    previous_deps.remove(&state.object);
    new_deps.insert(state.object);

    Ok(())
}

pub fn on_keyboard_for_replace_parent_3d(
    In(code): In<KeyCode>,
) -> SelectionNodeResult {
    if matches!(code, KeyCode::Escape) {
        // Simply exit the workflow if the user presses esc
        return Err(None);
    }

    Ok(())
}
