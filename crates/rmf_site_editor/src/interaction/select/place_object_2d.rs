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
    site::{ModelInstance, ModelLoader},
};
use bevy::prelude::ButtonInput;

pub const PLACE_OBJECT_2D_MODE_LABEL: &'static str = "place_object_2d";

pub fn spawn_place_object_2d_workflow(app: &mut App) -> Service<Option<Entity>, ()> {
    let setup = app.spawn_service(place_object_2d_setup.into_blocking_service());
    let find_position = app.spawn_continuous_service(Update, place_object_2d_find_placement);
    let placement_chosen = app.spawn_service(on_placement_chosen_2d.into_blocking_service());
    let handle_key_code =
        app.spawn_service(on_keyboard_for_place_object_2d.into_blocking_service());
    let cleanup = app.spawn_service(place_object_2d_cleanup.into_blocking_service());

    let keyboard_just_pressed = app
        .world()
        .resource::<KeyboardServices>()
        .keyboard_just_pressed;

    app.world_mut()
        .spawn_io_workflow(build_place_object_2d_workflow(
            setup,
            find_position,
            placement_chosen,
            handle_key_code,
            cleanup,
            keyboard_just_pressed,
        ))
}

pub fn build_place_object_2d_workflow(
    setup: Service<BufferKey<PlaceObject2d>, SelectionNodeResult>,
    find_placement: Service<(), Transform>,
    placement_chosen: Service<(Transform, BufferKey<PlaceObject2d>), SelectionNodeResult>,
    handle_key_code: Service<KeyCode, SelectionNodeResult>,
    cleanup: Service<BufferKey<PlaceObject2d>, ()>,
    keyboard_just_pressed: Service<(), (), StreamOf<KeyCode>>,
) -> impl FnOnce(Scope<Option<Entity>, ()>, &mut Builder) {
    move |scope, builder| {
        let buffer = builder.create_buffer::<PlaceObject2d>(BufferSettings::keep_last(1));

        let setup_finished = scope
            .input
            .chain(builder)
            .then(extract_selector_input::<PlaceObject2d>.into_blocking_callback())
            .branch_for_err(|err| err.connect(scope.terminate))
            .cancel_on_none()
            .then_push(buffer)
            .then_access(buffer)
            .then(setup)
            .branch_for_err(|err| err.map_block(print_if_err).connect(scope.terminate))
            .output()
            .fork_clone(builder);

        setup_finished
            .clone_chain(builder)
            .then(find_placement)
            .with_access(buffer)
            .then(placement_chosen)
            .fork_result(
                |ok| ok.connect(scope.terminate),
                |err| err.map_block(print_if_err).connect(scope.terminate),
            );

        let keyboard_node = setup_finished
            .clone_chain(builder)
            .then_node(keyboard_just_pressed);
        keyboard_node
            .streams
            .chain(builder)
            .inner()
            .then(handle_key_code)
            .fork_result(
                |ok| ok.connect(scope.terminate),
                |err| err.map_block(print_if_err).connect(scope.terminate),
            );

        builder.on_cleanup(buffer, move |scope, builder| {
            scope
                .input
                .chain(builder)
                .then(cleanup)
                .connect(scope.terminate);
        });
    }
}

pub struct PlaceObject2d {
    pub object: ModelInstance<Entity>,
    pub level: Entity,
}

pub fn place_object_2d_setup(
    In(key): In<BufferKey<PlaceObject2d>>,
    mut access: BufferAccessMut<PlaceObject2d>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut commands: Commands,
    mut gizmo_blockers: ResMut<GizmoBlockers>,
    mut highlight: ResMut<HighlightAnchors>,
    mut model_loader: ModelLoader,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_buffer()?;

    cursor.set_model_instance_preview(&mut commands, &mut model_loader, Some(state.object.clone()));
    set_visibility(cursor.dagger, &mut visibility, false);
    set_visibility(cursor.halo, &mut visibility, false);

    highlight.0 = false;
    gizmo_blockers.selecting = true;

    cursor.add_mode(PLACE_OBJECT_2D_MODE_LABEL, &mut visibility);

    Ok(())
}

pub fn place_object_2d_cleanup(
    In(_): In<BufferKey<PlaceObject2d>>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut commands: Commands,
    mut gizmo_blockers: ResMut<GizmoBlockers>,
) {
    cursor.remove_preview(&mut commands);
    cursor.remove_mode(PLACE_OBJECT_2D_MODE_LABEL, &mut visibility);
    gizmo_blockers.selecting = false;
}

pub fn place_object_2d_find_placement(
    In(ContinuousService { key }): ContinuousServiceInput<(), Transform>,
    mut orders: ContinuousQuery<(), Transform>,
    cursor: Res<Cursor>,
    mut transforms: Query<&mut Transform>,
    intersect_ground_params: IntersectGroundPlaneParams,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    block_status: Res<PickBlockStatus>,
) {
    let Some(mut orders) = orders.get_mut(&key) else {
        return;
    };

    let Some(order) = orders.get_mut(0) else {
        return;
    };

    // TODO(@mxgrey): Consider allowing models to be snapped to existing objects
    // similar to how they can for the 3D object placement workflow. Either we
    // need to introduce parent frames to the 2D sites or just don't bother with
    // parenting.
    if let Some(intersection) = intersect_ground_params.ground_plane_intersection() {
        match transforms.get_mut(cursor.frame) {
            Ok(mut transform) => {
                *transform = intersection;
            }
            Err(err) => {
                error!("No cursor transform found: {err}");
            }
        }

        let clicked = mouse_button_input.just_pressed(MouseButton::Left);
        let blocked = block_status.blocked();

        if clicked && !blocked {
            order.respond(intersection);
        }
    } else {
        warn!("Unable to find a placement position. Try adjusting your camera angle.");
    }
}

pub fn on_keyboard_for_place_object_2d(In(key): In<KeyCode>) -> SelectionNodeResult {
    if matches!(key, KeyCode::Escape) {
        // Simply end the workflow if the escape key was pressed
        info!("Exiting 2D object placement");
        return Err(None);
    }

    Ok(())
}

pub fn on_placement_chosen_2d(
    In((placement, key)): In<(Transform, BufferKey<PlaceObject2d>)>,
    mut access: BufferAccessMut<PlaceObject2d>,
    mut model_loader: ModelLoader,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let mut state = access.pull().or_broken_state()?;

    state.object.pose = placement.into();
    model_loader
        .spawn_model_instance(state.level, state.object)
        .insert(Category::Model);

    Ok(())
}
