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
        Model, WorkcellModel, NameInSite, SiteID, AnchorBundle, Pose, Anchor,
        FrameMarker, NameInWorkcell, Dependents,
    },
    widgets::canvas_tooltips::CanvasTooltips,
    KeyboardServices,
    WorkspaceMarker,
};
use bevy::{
    prelude::{*, Input as UserInput},
    ecs::system::SystemParam,
};
use bevy_impulse::*;
use bevy_mod_raycast::deferred::RaycastSource;
use std::borrow::Cow;

pub const PLACE_OBJECT_3D_MODE_LABEL: &'static str = "place_object_3d";

#[derive(Default)]
pub struct ObjectPlacementPlugin {}

impl Plugin for ObjectPlacementPlugin {
    fn build(&self, app: &mut App) {
        let services = ObjectPlacementServices::from_app(app);
        app.insert_resource(services);
    }
}


#[derive(Resource, Clone, Copy)]
pub struct ObjectPlacementServices {
    pub place_object_3d: Service<Option<Entity>, ()>,
}

impl ObjectPlacementServices {
    pub fn from_app(app: &mut App) -> Self {
        let setup = app.spawn_service(place_object_3d_setup.into_blocking_service());
        let find_position = app.spawn_continuous_service(Update, place_object_3d_find_placement);
        let placement_chosen = app.spawn_service(on_placement_chosen.into_blocking_service());
        let handle_key_code = app.spawn_service(on_keyboard_for_place_object_3d.into_blocking_service());
        let cleanup = app.spawn_service(place_object_3d_cleanup.into_blocking_service());
        let hover_service = app.spawn_continuous_service(
            Update,
            hover_service::<PlaceObject3dFilter>
            .configure(|config: SystemConfigs|
                config.in_set(SelectionServiceStages::Hover)
            ),
        );
        let select_service = app.spawn_continuous_service(
            Update,
            select_service::<PlaceObject3dFilter>
            .configure(|config: SystemConfigs|
                config.in_set(SelectionServiceStages::Select)
            ),
        );
        let keyboard_just_pressed = app.world.resource::<KeyboardServices>()
            .keyboard_just_pressed;

        let place_object_3d = app.world.spawn_io_workflow(build_place_object_3d_workflow(
            setup,
            find_position,
            placement_chosen,
            handle_key_code,
            cleanup,
            hover_service.optional_stream_cast(),
            select_service.optional_stream_cast(),
            keyboard_just_pressed,
        ));

        Self { place_object_3d }
    }
}

#[derive(SystemParam)]
pub struct ObjectPlacement<'w, 's> {
    pub services: Res<'w, ObjectPlacementServices>,
    pub run: EventWriter<'w, RunSelector>,
    pub commands: Commands<'w, 's>,
}

impl<'w, 's> ObjectPlacement<'w, 's> {
    pub fn place_object_3d(
        &mut self,
        object: PlaceableObject,
        parent: Option<Entity>,
        workspace: Entity,
    ) {
        let state = self.commands.spawn(SelectorInput(PlaceObject3d { object, parent, workspace })).id();
        self.run.send(RunSelector {
            selector: self.services.place_object_3d,
            input: Some(state),
        });
    }
}

pub fn build_place_object_3d_workflow(
    setup: Service<BufferKey<PlaceObject3d>, SelectionNodeResult>,
    find_position: Service<BufferKey<PlaceObject3d>, Transform>,
    placement_chosen: Service<(Transform, BufferKey<PlaceObject3d>), SelectionNodeResult>,
    handle_key_code: Service<(KeyCode, BufferKey<PlaceObject3d>), SelectionNodeResult>,
    cleanup: Service<BufferKey<PlaceObject3d>, SelectionNodeResult>,
    // Used to manage highlighting prospective parent frames
    hover_service: Service<(), ()>,
    // Used to manage highlighting the current parent frame
    select_service: Service<(), ()>,
    keyboard_just_pressed: Service<(), (), StreamOf<KeyCode>>,
) -> impl FnOnce(Scope<Option<Entity>, ()>, &mut Builder) {
    move |scope, builder| {

        let buffer = builder.create_buffer::<PlaceObject3d>(BufferSettings::keep_last(1));

        let begin_input_services = scope.input.chain(builder)
            .then(extract_selector_input::<PlaceObject3d>.into_blocking_callback())
            .branch_for_err(|chain: Chain<_>| chain.connect(scope.terminate))
            .cancel_on_none()
            .then_access(buffer)
            .then(setup)
            .branch_for_err(|err| err.map_block(print_if_err).connect(scope.terminate))
            .output()
            .fork_clone(builder);

        begin_input_services.clone_chain(builder)
            .then_access(buffer)
            .then(find_position)
            .with_access(buffer)
            .then(placement_chosen)
            .fork_result(
                |ok| ok.connect(scope.terminate),
                |err| err.map_block(print_if_err).connect(scope.terminate),
            );

        begin_input_services.clone_chain(builder)
            .then(hover_service)
            .connect(scope.terminate);

        begin_input_services.clone_chain(builder)
            .then(select_service)
            .connect(scope.terminate);

        let keyboard = begin_input_services.clone_chain(builder)
            .then_node(keyboard_just_pressed);
        keyboard.streams.chain(builder)
            .inner()
            .with_access(buffer)
            .then(handle_key_code)
            .dispose_on_ok()
            .map_block(print_if_err)
            .connect(scope.terminate);

        builder.on_cleanup(buffer, move |scope, builder| {
            scope.input.chain(builder)
                .then(cleanup)
                .fork_result(
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

#[derive(Clone)]
pub enum PlaceableObject {
    Model(Model),
    Anchor,
    VisualMesh(WorkcellModel),
    CollisionMesh(WorkcellModel),
}

pub fn place_object_3d_setup(
    In(key): In<BufferKey<PlaceObject3d>>,
    mut access: BufferAccessMut<PlaceObject3d>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut commands: Commands,
    mut select: EventWriter<Select>,
    mut highlight: ResMut<HighlightAnchors>,
    mut filter: PlaceObject3dFilter,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.newest_mut().or_broken_buffer()?;

    match &state.object {
        PlaceableObject::Anchor => {
            // Make the anchor placement component of the cursor visible
            set_visibility(cursor.frame_placement, &mut visibility, true);
        }
        PlaceableObject::Model(m) => {
            // Spawn the model as a child of the cursor
            cursor.set_model_preview(&mut commands, Some(m.clone()));
        }
        PlaceableObject::VisualMesh(m) | PlaceableObject::CollisionMesh(m) => {
            // Spawn the model as a child of the cursor
            cursor.set_workcell_model_preview(&mut commands, Some(m.clone()));
        }
    }

    if let Some(parent) = state.parent {
        let parent = filter.filter_select(parent);
        select.send(Select::new(parent));
        state.parent = parent;
    }

    highlight.0 = true;

    cursor.add_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);

    Ok(())
}

pub fn place_object_3d_cleanup(
    In(_): In<BufferKey<PlaceObject3d>>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut commands: Commands,
    mut highlight: ResMut<HighlightAnchors>,
) -> SelectionNodeResult {
    cursor.remove_preview(&mut commands);
    cursor.remove_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);
    set_visibility(cursor.frame_placement, &mut visibility, false);
    highlight.0 = false;

    Ok(())
}

pub fn place_object_3d_find_placement(
    In(ContinuousService { key: srv_key }): ContinuousServiceInput<BufferKey<PlaceObject3d>, Transform>,
    mut orders: ContinuousQuery<BufferKey<PlaceObject3d>, Transform>,
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
    mut select: EventWriter<Select>,
    mouse_button_input: Res<UserInput<MouseButton>>,
    blockers: Option<Res<PickingBlockers>>,
    meta: Query<(Option<&'static NameInSite>, Option<&'static SiteID>)>,
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
        tooltips.tips.push(Cow::Borrowed("Esc: deselect current parent"));
    }

    let project_to_plane = keyboard_input.any_pressed([KeyCode::ShiftLeft, KeyCode::ShiftRight]);

    let mut transform = match transforms.get_mut(cursor.frame) {
        Ok(transform) => transform,
        Err(_) => {
            error!("No cursor transform found");
            return;
        }
    };

    let Ok(source) = raycast_sources.get_single() else {
        return;
    };

    // Check if there is an intersection to a mesh, if there isn't fallback to ground plane
    let mut intersection: Option<Transform> = None;
    let mut new_hover = None;
    let mut select_new_parent = false;
    if state.parent.is_none() || !project_to_plane {
        for (e, i) in source.intersections() {
            let Some(e) = filter.filter_select(*e) else {
                continue;
            };

            if let Some(parent) = state.parent {
                if e == parent {
                    new_hover = Some(parent);
                    cursor.add_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);
                    tooltips.tips.push(Cow::Borrowed("Click to place"));
                    tooltips.tips.push(Cow::Borrowed("+Shift: Project to parent frame"));
                    intersection = Some(
                        Transform::from_translation(i.position())
                        .looking_to(i.normal(), Vec3::Z)
                    );
                    break;
                }
            } else {
                new_hover = Some(e);
                select_new_parent = true;
                cursor.remove_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);
                tooltips.tips.push(Cow::Borrowed("Click to set as parent"));
                tooltips.tips.push(Cow::Borrowed("+Shift: Project to ground plane"));
                break;
            }
        }
    }

    if new_hover != hovering.0 {
        hover.send(Hover(new_hover));
    }

    if !select_new_parent {
        let intersection = intersection.or_else(|| {
            if let Some(parent) = state.parent {
                tooltips.tips.push(Cow::Borrowed("Click to place"));
                cursor.add_mode(PLACE_OBJECT_3D_MODE_LABEL, &mut visibility);
                intersect_ground_params.frame_plane_intersection(parent)
            } else {
                tooltips.tips.push(Cow::Borrowed("Click to place"));
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
                select.send(Select::new(Some(new_parent)));
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
    frames: Query<'w, 's, (), With<FrameMarker>>,
    workspaces: Query<'w, 's, (), With<WorkspaceMarker>>,
    parents: Query<'w, 's, &'static Parent>,
}

impl<'w, 's> PlaceObject3dFilter<'w, 's> {
    pub fn find_frame(&self, mut e: Entity) -> Option<Entity> {
        loop {
            if self.frames.contains(e) {
                return Some(e);
            }

            if self.workspaces.contains(e) {
                return Some(e);
            }

            if let Ok(parent) = self.parents.get(e) {
                e = parent.get();
            } else {
                return None;
            }
        }
    }
}

impl<'w, 's> SelectionFilter for PlaceObject3dFilter<'w, 's> {
    fn filter_pick(&mut self, target: Entity) -> Option<Entity> {
        self.inspect.filter_pick(target).and_then(|e| self.find_frame(e))
    }

    fn filter_select(&mut self, target: Entity) -> Option<Entity> {
        self.inspect.filter_select(target).and_then(|e| self.find_frame(e))
    }

    fn on_click(&mut self, _: Hover) -> Option<Select> {
        // Do nothing, clicking is handled by place_object_3d_find_position.
        // The hover_service doesn't have access to the state of the workflow
        // so it can't judge whether to select a new parent or not.
        None
    }
}

pub fn on_keyboard_for_place_object_3d(
    In((button, key)): In<(KeyCode, BufferKey<PlaceObject3d>)>,
    mut access: BufferAccessMut<PlaceObject3d>,
    mut select: EventWriter<Select>,
) -> SelectionNodeResult {
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
        select.send(Select::new(None));
    } else {
        info!("Exiting 3D object placement");
        return Err(None);
    }

    Ok(())
}

pub fn on_placement_chosen(
    In((placement, key)): In<(Transform, BufferKey<PlaceObject3d>)>,
    mut access: BufferAccessMut<PlaceObject3d>,
    mut commands: Commands,
    mut dependents: Query<&mut Dependents>,
    global_tfs: Query<&GlobalTransform>,
) -> SelectionNodeResult {
    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.pull().or_broken_state()?;

    // let pose: Pose = placement.into();
    let pose: Pose = if let Some(parent) = state.parent {
        let parent_tf = global_tfs.get(parent).or_broken_query()?;
        let inv_tf = parent_tf.affine().inverse();
        let placement_tf = placement.compute_affine();
        Transform::from_matrix((inv_tf * placement_tf).into()).into()
    } else {
        placement.into()
    };

    let id = match state.object {
        PlaceableObject::Anchor => {
            commands.spawn((
                AnchorBundle::new(Anchor::Pose3D(pose)),
                FrameMarker,
                NameInWorkcell("Unnamed".to_string()),
            ))
            .id()
        }
        PlaceableObject::Model(object) => {
            let model_id = commands.spawn(object).id();
            // Create a parent anchor to contain the new model in
            commands.spawn((
                AnchorBundle::new(Anchor::Pose3D(pose))
                    .dependents(Dependents::single(model_id)),
                FrameMarker,
                NameInWorkcell("model_root".to_string()),
            ))
            .add_child(model_id)
            .id()
        }
        PlaceableObject::VisualMesh(mut object) => {
            object.pose = pose;
            let mut cmd = commands.spawn(VisualMeshMarker);
            object.add_bevy_components(&mut cmd);
            cmd.id()
        }
        PlaceableObject::CollisionMesh(mut object) => {
            object.pose = pose;
            let mut cmd = commands.spawn(CollisionMeshMarker);
            object.add_bevy_components(&mut cmd);
            cmd.id()
        }
    };

    commands.entity(id).set_parent(state.parent.unwrap_or(state.workspace));
    if let Some(parent) = state.parent {
        if let Ok(mut deps) = dependents.get_mut(parent) {
            deps.insert(id);
        }
    }

    Ok(())
}
