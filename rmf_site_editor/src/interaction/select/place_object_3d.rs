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
    site::{
        Anchor, AnchorBundle, Dependents, FrameMarker, Model, ModelLoadingRequest,
        ModelSpawningExt, NameInWorkcell, Pending, SiteID, WorkcellModel,
    },
    widgets::canvas_tooltips::CanvasTooltips,
    workcell::flatten_loaded_model_hierarchy,
};
use bevy::{
    ecs::system::{EntityCommands, SystemParam},
    prelude::Input as UserInput,
};
use bevy_mod_raycast::deferred::RaycastSource;
use std::borrow::Cow;

pub const PLACE_OBJECT_3D_MODE_LABEL: &'static str = "place_object_3d";

pub fn spawn_place_object_3d_workflow(
    hover_service: Service<(), (), Hover>,
    app: &mut App,
) -> Service<Option<Entity>, ()> {
    let setup = app.spawn_service(place_object_3d_setup);
    let find_position = app.spawn_continuous_service(Update, place_object_3d_find_placement);
    let placement_chosen = app.spawn_service(on_placement_chosen_3d.into_blocking_service());
    let handle_key_code = app.spawn_service(on_keyboard_for_place_object_3d);
    let cleanup = app.spawn_service(place_object_3d_cleanup.into_blocking_service());
    let selection_update = app.world.resource::<InspectorService>().selection_update;
    let keyboard_just_pressed = app
        .world
        .resource::<KeyboardServices>()
        .keyboard_just_pressed;

    app.world.spawn_io_workflow(build_place_object_3d_workflow(
        setup,
        find_position,
        placement_chosen,
        handle_key_code,
        cleanup,
        hover_service.optional_stream_cast(),
        selection_update,
        keyboard_just_pressed,
    ))
}

pub fn build_place_object_3d_workflow(
    setup: Service<BufferKey<PlaceObject3d>, SelectionNodeResult, Select>,
    find_placement: Service<BufferKey<PlaceObject3d>, Transform, Select>,
    placement_chosen: Service<(Transform, BufferKey<PlaceObject3d>), SelectionNodeResult>,
    handle_key_code: Service<(KeyCode, BufferKey<PlaceObject3d>), SelectionNodeResult, Select>,
    cleanup: Service<BufferKey<PlaceObject3d>, SelectionNodeResult>,
    // Used to manage highlighting prospective parent frames
    hover_service: Service<(), ()>,
    // Used to manage highlighting the current parent frame
    selection_update: Service<Select, ()>,
    keyboard_just_pressed: Service<(), (), StreamOf<KeyCode>>,
) -> impl FnOnce(Scope<Option<Entity>, ()>, &mut Builder) {
    move |scope, builder| {
        let buffer = builder.create_buffer::<PlaceObject3d>(BufferSettings::keep_last(1));
        let selection_update_node = builder.create_node(selection_update);
        let setup_node = scope
            .input
            .chain(builder)
            .then(extract_selector_input::<PlaceObject3d>.into_blocking_callback())
            .branch_for_err(|err| err.connect(scope.terminate))
            .cancel_on_none()
            .then_push(buffer)
            .then_access(buffer)
            .then_node(setup);

        builder.connect(setup_node.streams, selection_update_node.input);

        let begin_input_services = setup_node
            .output
            .chain(builder)
            .branch_for_err(|err| err.map_block(print_if_err).connect(scope.terminate))
            .output()
            .fork_clone(builder);

        let find_placement_node = begin_input_services
            .clone_chain(builder)
            .then_access(buffer)
            .then_node(find_placement);

        find_placement_node
            .output
            .chain(builder)
            .with_access(buffer)
            .then(placement_chosen)
            .fork_result(
                |ok| ok.connect(scope.terminate),
                |err| err.map_block(print_if_err).connect(scope.terminate),
            );

        builder.connect(find_placement_node.streams, selection_update_node.input);

        begin_input_services
            .clone_chain(builder)
            .then(hover_service)
            .connect(scope.terminate);

        let keyboard = begin_input_services
            .clone_chain(builder)
            .then_node(keyboard_just_pressed);
        let handle_key_node = keyboard
            .streams
            .chain(builder)
            .inner()
            .with_access(buffer)
            .then_node(handle_key_code);

        handle_key_node
            .output
            .chain(builder)
            .dispose_on_ok()
            .map_block(print_if_err)
            .connect(scope.terminate);

        builder.connect(handle_key_node.streams, selection_update_node.input);

        builder.on_cleanup(buffer, move |scope, builder| {
            scope.input.chain(builder).then(cleanup).fork_result(
                |ok| ok.connect(scope.terminate),
                |err| err.map_block(print_if_err).connect(scope.terminate),
            );
        });
    }
}

pub struct PlaceObject3d {
    pub object: PlaceableObject,
    pub parent: Option<Entity>,
    pub workspace: Entity,
}

#[derive(Clone, Debug)]
pub enum PlaceableObject {
    Model(Model),
    Anchor,
    VisualMesh(Model),
    CollisionMesh(Model),
}

pub fn place_object_3d_setup(
    In(srv): BlockingServiceInput<BufferKey<PlaceObject3d>, Select>,
    mut access: BufferAccessMut<PlaceObject3d>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut commands: Commands,
    mut highlight: ResMut<HighlightAnchors>,
    mut filter: PlaceObject3dFilter,
    mut gizmo_blockers: ResMut<GizmoBlockers>,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&srv.request).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_buffer()?;

    match &state.object {
        PlaceableObject::Anchor => {
            // Make the anchor placement component of the cursor visible
            set_visibility(cursor.frame_placement, &mut visibility, true);
            set_visibility(cursor.dagger, &mut visibility, true);
            set_visibility(cursor.halo, &mut visibility, true);
        }
        PlaceableObject::Model(m)
        | PlaceableObject::VisualMesh(m)
        | PlaceableObject::CollisionMesh(m) => {
            // Spawn the model as a child of the cursor
            cursor.set_model_preview(&mut commands, Some(m.clone()));
            set_visibility(cursor.dagger, &mut visibility, false);
            set_visibility(cursor.halo, &mut visibility, false);
        }
    }

    if let Some(parent) = state.parent {
        let parent = filter.filter_select(parent);
        state.parent = parent;
    }
    srv.streams.send(Select::new(state.parent));

    highlight.0 = true;
    gizmo_blockers.selecting = true;

    cursor.add_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);

    Ok(())
}

pub fn place_object_3d_cleanup(
    In(_): In<BufferKey<PlaceObject3d>>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut commands: Commands,
    mut highlight: ResMut<HighlightAnchors>,
    mut gizmo_blockers: ResMut<GizmoBlockers>,
) -> SelectionNodeResult {
    cursor.remove_preview(&mut commands);
    cursor.remove_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);
    set_visibility(cursor.frame_placement, &mut visibility, false);
    highlight.0 = false;
    gizmo_blockers.selecting = false;

    Ok(())
}

pub fn place_object_3d_find_placement(
    In(ContinuousService { key: srv_key }): ContinuousServiceInput<
        BufferKey<PlaceObject3d>,
        Transform,
        Select,
    >,
    mut orders: ContinuousQuery<BufferKey<PlaceObject3d>, Transform, Select>,
    mut buffer: BufferAccessMut<PlaceObject3d>,
    mut cursor: ResMut<Cursor>,
    raycast_sources: Query<&RaycastSource<SiteRaycastSet>>,
    mut transforms: Query<&mut Transform>,
    intersect_ground_params: IntersectGroundPlaneParams,
    mut visibility: Query<&mut Visibility>,
    mut tooltips: ResMut<CanvasTooltips>,
    keyboard_input: Res<UserInput<KeyCode>>,
    mut hover: EventWriter<Hover>,
    hovering: Res<Hovering>,
    mouse_button_input: Res<UserInput<MouseButton>>,
    blockers: Option<Res<PickingBlockers>>,
    meta: Query<(Option<&'static NameInWorkcell>, Option<&'static SiteID>)>,
    mut filter: PlaceObject3dFilter,
) {
    let Some(mut orders) = orders.get_mut(&srv_key) else {
        return;
    };

    let Some(order) = orders.get_mut(0) else {
        return;
    };

    let key = order.request();
    let Ok(mut buffer) = buffer.get_mut(key) else {
        error!("Unable to retrieve buffer in place_object_3d_cursor_transform");
        return;
    };
    let Some(state) = buffer.newest_mut() else {
        error!("Missing state in place_object_3d_cursor_transform");
        return;
    };

    if state.parent.is_some() {
        tooltips.add(Cow::Borrowed("Esc: deselect current parent"));
    }

    let project_to_plane = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    let mut transform = match transforms.get_mut(cursor.frame) {
        Ok(transform) => transform,
        Err(err) => {
            error!("No cursor transform found: {err}");
            return;
        }
    };

    let Ok(source) = raycast_sources.get_single() else {
        return;
    };

    // Check if there is an intersection with a mesh
    let mut intersection: Option<Transform> = None;
    let mut new_hover = None;
    let mut select_new_parent = false;
    if !project_to_plane {
        for (e, i) in source.intersections() {
            let Some(e) = filter.filter_pick(*e) else {
                continue;
            };

            if let Some(parent) = state.parent {
                if e == parent {
                    new_hover = Some(parent);
                    cursor.add_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);
                    tooltips.add(Cow::Borrowed("Click to place"));
                    tooltips.add(Cow::Borrowed("+Shift: Project to parent frame"));

                    // Don't use the intersection with the parent if the parent
                    // is an anchor because that results in silly orientations
                    // which the user probably does not want.
                    if !filter.anchors.contains(e) {
                        intersection = Some(
                            Transform::from_translation(i.position())
                                .with_rotation(aligned_z_axis(i.normal())),
                        );
                    }
                    break;
                }
            } else {
                new_hover = Some(e);
                select_new_parent = true;
                cursor.remove_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);
                tooltips.add(Cow::Borrowed("Click to set as parent"));
                tooltips.add(Cow::Borrowed("+Shift: Project to ground plane"));
                break;
            }
        }
    } else {
        cursor.add_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);
    }

    if new_hover != hovering.0 {
        hover.send(Hover(new_hover));
    }

    if !select_new_parent {
        intersection = intersection.or_else(|| {
            if let Some(parent) = state.parent {
                tooltips.add(Cow::Borrowed("Click to place"));
                cursor.add_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);
                intersect_ground_params.frame_plane_intersection(parent)
            } else {
                tooltips.add(Cow::Borrowed("Click to place"));
                cursor.add_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);
                intersect_ground_params.ground_plane_intersection()
            }
        });

        if let Some(intersection) = intersection {
            *transform = intersection;
        }
    }

    let clicked = mouse_button_input.just_pressed(MouseButton::Left);
    let blocked = blockers.filter(|x| x.blocking()).is_some();
    if clicked && !blocked {
        if select_new_parent {
            if let Some(new_parent) = new_hover {
                state.parent = Some(new_parent);
                order.streams().send(Select::new(Some(new_parent)));
                if let Ok((name, id)) = meta.get(new_parent) {
                    let id = id.map(|id| id.0.to_string());
                    info!(
                        "Placing object in the frame of [{}], id: {}",
                        name.map(|name| name.0.as_str()).unwrap_or("<name unset>"),
                        id.as_ref().map(|id| id.as_str()).unwrap_or("*"),
                    );
                }
            }
        } else {
            if let Some(intersection) = intersection {
                // The user is choosing a location to place the object.
                order.respond(intersection);
            } else {
                warn!("Unable to find a placement position. Try adjusting your camera angle.");
            }
        }
    }
}

#[derive(SystemParam)]
pub struct PlaceObject3dFilter<'w, 's> {
    inspect: InspectorFilter<'w, 's>,
    ignore: Query<'w, 's, (), Or<(With<Preview>, With<Pending>)>>,
    // We aren't using this in the filter functions, we're sneaking this query
    // into this system param to skirt around the 16-parameter limit for
    // place_object_3d_find_placement
    anchors: Query<'w, 's, (), With<Anchor>>,
}

impl<'w, 's> SelectionFilter for PlaceObject3dFilter<'w, 's> {
    fn filter_pick(&mut self, target: Entity) -> Option<Entity> {
        let e = self.inspect.filter_pick(target);

        if let Some(e) = e {
            if self.ignore.contains(e) {
                return None;
            }
        }
        e
    }

    fn filter_select(&mut self, target: Entity) -> Option<Entity> {
        self.inspect.filter_select(target)
    }

    fn on_click(&mut self, _: Hover) -> Option<Select> {
        // Do nothing, clicking is handled by place_object_3d_find_position.
        // The hover_service doesn't have access to the state of the workflow
        // so it can't judge whether to select a new parent or not.
        None
    }
}

pub fn on_keyboard_for_place_object_3d(
    In(srv): BlockingServiceInput<(KeyCode, BufferKey<PlaceObject3d>), Select>,
    mut access: BufferAccessMut<PlaceObject3d>,
) -> SelectionNodeResult {
    let (button, key) = srv.request;
    if !matches!(button, KeyCode::Escape) {
        // The button was not the escape key, so there's nothing for us to do
        // here.
        return Ok(());
    }

    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_state()?;

    if state.parent.is_some() {
        // Remove the parent
        info!("Placing object in the ground plane");
        state.parent = None;
        srv.streams.send(Select::new(None));
    } else {
        info!("Exiting 3D object placement");
        return Err(None);
    }

    Ok(())
}

pub fn on_placement_chosen_3d(
    In((placement, key)): In<(Transform, BufferKey<PlaceObject3d>)>,
    mut access: BufferAccessMut<PlaceObject3d>,
    mut commands: Commands,
    mut dependents: Query<&mut Dependents>,
    global_tfs: Query<&GlobalTransform>,
    parents: Query<&Parent>,
    frames: Query<(), With<FrameMarker>>,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.pull().or_broken_state()?;

    let parent = state
        .parent
        .and_then(|p| {
            if frames.contains(p) {
                Some(p)
            } else {
                // The selected parent is not a frame, so find its first ancestor
                // that contains a FrameMarker
                AncestorIter::new(&parents, p).find(|e| frames.contains(*e))
            }
        })
        .unwrap_or(state.workspace);

    let parent_tf = global_tfs.get(parent).or_broken_query()?;
    let inv_tf = parent_tf.affine().inverse();
    let placement_tf = placement.compute_affine();
    let pose = Transform::from_matrix((inv_tf * placement_tf).into()).into();

    let cb = flatten_loaded_model_hierarchy.into_blocking_callback();
    let add_model_components = |object: Model, mut cmd: EntityCommands| {
        cmd.insert((
            NameInWorkcell(object.name.0),
            object.pose,
            object.is_static,
            object.scale,
        ));
    };
    let id = match state.object {
        PlaceableObject::Anchor => commands
            .spawn((
                AnchorBundle::new(Anchor::Pose3D(pose)),
                FrameMarker,
                NameInWorkcell("Unnamed".to_string()),
            ))
            .id(),
        PlaceableObject::Model(object) => {
            // TODO(luca) check if we should have a custom then_commands here
            let model_id = commands.spawn(VisualCue::outline()).id();
            let req = ModelLoadingRequest::new(model_id, object.source.clone())
                .then(cb)
                .then_command(move |cmd: EntityCommands| {
                    add_model_components(object, cmd);
                });
            commands.spawn_model(req);
            // Create a parent anchor to contain the new model in
            commands
                .spawn((
                    AnchorBundle::new(Anchor::Pose3D(pose))
                        .dependents(Dependents::single(model_id)),
                    FrameMarker,
                    NameInWorkcell("model_root".to_string()),
                ))
                .add_child(model_id)
                .id()
        }
        PlaceableObject::VisualMesh(mut object) => {
            let id = commands.spawn((VisualMeshMarker, Category::Visual)).id();
            object.pose = pose;
            let req = ModelLoadingRequest::new(id, object.source.clone())
                .then(cb)
                .then_command(move |cmd: EntityCommands| {
                    add_model_components(object, cmd);
                });
            commands.spawn_model(req);
            id
        }
        PlaceableObject::CollisionMesh(mut object) => {
            let id = commands
                .spawn((CollisionMeshMarker, Category::Collision))
                .id();
            object.pose = pose;
            let req = ModelLoadingRequest::new(id, object.source.clone())
                .then(cb)
                .then_command(move |cmd: EntityCommands| {
                    add_model_components(object, cmd);
                });
            commands.spawn_model(req);
            id
        }
    };

    commands
        .get_entity(id)
        .or_broken_query()?
        .set_parent(parent);
    if let Ok(mut deps) = dependents.get_mut(parent) {
        deps.insert(id);
    }

    Ok(())
}
