use bevy_ecs::{
    prelude::*,
    system::{StaticSystemParam, SystemParam},
};
use bevy_impulse::*;
use bevy_input::prelude::*;
use bevy_math::prelude::*;
use bevy_picking::pointer::{PointerId, PointerInteraction};
use bevy_transform::components::Transform;
use rmf_site_camera::*;
// use rmf_site_format::Site::{Group, ModelMarker};
use tracing::warn;
use web_time::Instant;

use crate::*;

const DOUBLE_CLICK_DURATION_MILLISECONDS: u128 = 500;

pub fn process_new_selector(
    In(key): In<BufferKey<RunSelector>>,
    mut access: BufferAccessMut<RunSelector>,
) -> Option<RunSelector> {
    let Ok(mut buffer) = access.get_mut(&key) else {
        return None;
    };

    let output = buffer.pull();
    if output.is_some() {
        // We should lock the gate while the trim is going on so we can't have
        // multiple new selectors trying to start at the same time
        buffer.close_gate();
    }

    output
}

pub fn selection_update(
    In(BlockingService {
        request: Select(new_selection),
        ..
    }): BlockingServiceInput<Select>,
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    // If SHIFT is held down, we do not de-select the previous element, we add the selection candidate to the Selection resource
    if let Some(new_selection) = new_selection.map(|s| s.candidate) {
        if !selection.0.contains(&new_selection) {
            if keyboard_input.pressed(KeyCode::ShiftLeft) {
                // todo(@johntgz) Adding entities to the current selection is only supported for Model and ModelInstances only, we should filter it using With<ModelMarker>, Without<Group>
                // Add a query
                if let Ok(mut selected) = selected.get_mut(new_selection) {
                    selected.is_selected = true;
                    selection.0.insert(new_selection);
                }
            } else {
                // Only one entity can be selected. Un-select all previous selections and add entity to current selection.
                selection.0.iter().for_each(|previous_selection| {
                    if let Ok(mut selected) = selected.get_mut(*previous_selection) {
                        selected.is_selected = false;
                    }
                });
                selection.0.clear();

                if let Ok(mut selected) = selected.get_mut(new_selection) {
                    selected.is_selected = true;
                }
                selection.0.insert(new_selection);
            }
        }
    }
}

/// A continuous service that generates Hover events based on ongoing mouse
/// picking activities.
///
/// This service should be activated in a workflow when you want user mouse
/// interactions to generate Hover events that are compatible with the Inspector
/// interaction mode. This allows any "site element" (an item that has a
/// [`Category`]) to be picked by the user's mouse for hovering and then
/// selecting.
///
/// This will not emit any streams or ever yield a response. Its work is done
/// entirely in the background of the workflow. To receive updates on hover
/// events, you must also run [`inspector_hover_service`] and watch its [`Hover`]
/// stream. You should also run [`inspector_select_service`] for [`Select`]
/// streams.
///
/// [`Category`]: rmf_site_format::Category
pub fn picking_service<Filter: SystemParam + 'static>(
    In(ContinuousService { key }): ContinuousServiceInput<(), ()>,
    orders: ContinuousQuery<(), ()>,
    mut picks: EventReader<ChangePick>,
    mut hover: EventWriter<Hover>,
    filter: StaticSystemParam<Filter>,
) where
    for<'w, 's> Filter::Item<'w, 's>: SelectionFilter,
{
    let Some(orders) = orders.view(&key) else {
        return;
    };

    if orders.is_empty() {
        // Nothing is asking for this service to run, so skip it
        return;
    }

    let mut filter = filter.into_inner();

    if let Some(pick) = picks.read().last() {
        hover.write(Hover(
            pick.to
                .and_then(|change_pick_to| filter.filter_pick(change_pick_to)),
        ));
    }
}

pub fn make_selectable_entities_pickable(
    mut commands: Commands,
    new_selectables: Query<&Selectable, Added<Selectable>>,
    targets: Query<(Option<&Hovered>, Option<&Selected>)>,
) {
    for selectable in &new_selectables {
        if let Ok((hovered, selected)) = targets.get(selectable.element) {
            if hovered.is_none() {
                commands
                    .entity(selectable.element)
                    .insert(Hovered::default());
            }

            if selected.is_none() {
                commands
                    .entity(selectable.element)
                    .insert(Selected::default());
            }
        }
    }
}

/// A continuous service that processes [`Hover`] events, updates the World, and
/// issues out a [`Hover`] stream.
///
/// This service should be activated in a workflow when you want to process
/// [`Hover`] events. This will stream out [`Hover`] events for your workflow to
/// process while also making sure the components of entities in the application
/// are kept up to date. Its Hover events are suitable for the Inspector
/// interaction mode.
///
/// This will never yield a response to any requests, only stream out events
/// until cleanup.
///
/// This is meant to be used with
/// - [`inspector_hover_picking`]
/// - [`inspector_select_service`]
pub fn hover_service<Filter: SystemParam + 'static>(
    In(ContinuousService { key }): ContinuousServiceInput<(), (), Hover>,
    mut orders: ContinuousQuery<(), (), Hover>,
    mut hovered: Query<&mut Hovered>,
    mut hovering: ResMut<Hovering>,
    mut hover: EventReader<Hover>,
    mouse_button_input: Res<ButtonInput<MouseButton>>,
    touch_input: Res<Touches>,
    mut select: EventWriter<Select>,
    block_status: Res<PickBlockStatus>,
    filter: StaticSystemParam<Filter>,
    selection_blockers: Res<SelectionBlockers>,
) where
    for<'w, 's> Filter::Item<'w, 's>: SelectionFilter,
{
    let Some(mut orders) = orders.get_mut(&key) else {
        return;
    };

    if orders.is_empty() {
        // Nothing is asking for this service to run
        return;
    }

    if selection_blockers.blocking() {
        return;
    }

    let mut filter = filter.into_inner();

    if let Some(new_hovered) = hover.read().last() {
        let new_hovered = new_hovered.0.and_then(|e| filter.filter_select(e));
        if hovering.0 != new_hovered {
            if let Some(previous_hovered) = hovering.0 {
                if let Ok(mut hovering) = hovered.get_mut(previous_hovered) {
                    hovering.is_hovered = false;
                }
            }

            if let Some(new_hovered) = new_hovered {
                if let Ok(mut hovering) = hovered.get_mut(new_hovered) {
                    hovering.is_hovered = true;
                }
            }

            hovering.0 = new_hovered;
            orders.for_each(|order| order.streams().send(Hover(new_hovered)));
        }
    }

    let clicked = mouse_button_input.just_pressed(MouseButton::Left)
        || touch_input.iter_just_pressed().next().is_some();

    let blocked = block_status.blocked();
    if clicked && !blocked {
        if let Some(new_select) = filter.on_click(Hover(hovering.0)) {
            select.write(new_select);
        }
    }
}

/// A continuous service that filters [`Select`] events and issues out a
/// [`Hover`] stream.
///
/// This complements [`hover_service`] and [`hover_picking`]
/// and is the final piece of the [`SelectionService`] workflow.
pub fn select_service<Filter: SystemParam + 'static>(
    In(ContinuousService { key }): ContinuousServiceInput<(), (), Select>,
    mut orders: ContinuousQuery<(), (), Select>,
    mut select: EventReader<Select>,
    filter: StaticSystemParam<Filter>,
    mut commands: Commands,
) where
    for<'w, 's> Filter::Item<'w, 's>: SelectionFilter,
{
    let Some(mut orders) = orders.get_mut(&key) else {
        return;
    };

    if orders.is_empty() {
        // Nothing is asking for this service to run
        return;
    }

    let mut filter = filter.into_inner();

    for selected in select.read() {
        let mut selected = *selected;
        if let Some(selected) = &mut selected.0 {
            match filter.filter_select(selected.candidate) {
                Some(candidate) => selected.candidate = candidate,
                None => {
                    // This request is being filtered out, we will not send it
                    // along at all.
                    if selected.provisional {
                        // The selection was provisional. Since we are not
                        // using it, we are responsible for despawning it.
                        if let Ok(mut entity_mut) = commands.get_entity(selected.candidate) {
                            entity_mut.despawn();
                        }
                    }
                    continue;
                }
            }
        }

        orders.for_each(|order| order.streams().send(selected));
    }
}

/// This is used to clear out the currently picked item at the start of a new
/// selection workflow to make sure the Hover events don't get lost during the
/// workflow switch.
pub fn refresh_picked(In(_): In<()>, mut picked: ResMut<Picked>) {
    picked.refresh = true;
}

/// This is used to clear out hoverings and selections from a workflow that is
/// cleaning up so that these properties don't spill over into other workflows.
pub fn clear_hover_select(
    In(_): In<()>,
    mut hovered: Query<&mut Hovered>,
    mut hovering: ResMut<Hovering>,
    mut selected: Query<&mut Selected>,
    selection: ResMut<Selection>,
) {
    if let Some(previous_hovering) = hovering.0.take() {
        if let Ok(mut hovered) = hovered.get_mut(previous_hovering) {
            hovered.is_hovered = false;
        }
    }

    for previous_selection in selection.0.iter() {
        if let Ok(mut selected) = selected.get_mut(*previous_selection) {
            selected.is_selected = false;
        }
    }
}

/// Update the virtual cursor (dagger and circle) transform while in inspector mode
pub fn inspector_cursor_transform(
    In(ContinuousService { key }): ContinuousServiceInput<(), ()>,
    orders: ContinuousQuery<(), ()>,
    mut cursor: Query<&mut Transform, With<CursorFrame>>,
    active_camera: ActiveCameraQuery,
    pointers: Query<(&PointerId, &PointerInteraction)>,
) {
    let Some(orders) = orders.view(&key) else {
        return;
    };

    if orders.is_empty() {
        return;
    }

    let Some((_, interactions)) = pointers.single().ok() else {
        return;
    };
    let Ok(active_camera) = active_camera_maybe(&active_camera) else {
        return;
    };
    let Some((position, normal)) = interactions
        .iter()
        .find(|(_, hit_data)| hit_data.camera == active_camera)
        .and_then(|(_, hit_data)| {
            hit_data
                .position
                .zip(hit_data.normal.and_then(|n| Dir3::new(n).ok()))
        })
    else {
        return;
    };

    let mut transform = match cursor.single_mut() {
        Ok(trans) => trans,
        Err(err) => {
            warn!("Could not get cursor transform. Reason: {:#?}", err);
            return;
        }
    };

    let ray = Ray3d::new(position, normal);
    *transform = Transform::from_matrix(Mat4::from_rotation_translation(
        Quat::from_rotation_arc(Vec3::new(0., 0., 1.), *ray.direction),
        ray.origin,
    ));
}

pub fn deselect_on_esc(In(code): In<KeyCode>, mut select: EventWriter<Select>) {
    if matches!(code, KeyCode::Escape) {
        select.write(Select::new(None));
    }
}

/// This builder function creates the high-level workflow that manages "selection"
/// behavior, which is largely driven by mouse interactions. "Selection" behaviors
/// determine how the application responds to the mouse cursor hovering over
/// objects in the scene and what happens when the mouse is clicked.
///
/// The default selection behavior is the "inspector" service which allows the
/// user to select objects in the scene so that their properties get displayed
/// in the inspector panel. The inspector service will automatically be run by
/// this workflow at startup.
///
/// When the user asks for some other type of mouse interaction to begin, such as
/// drawing walls and floors, or placing models in the scene, this workflow will
/// "trim" (stop) the inspector workflow and inject the new requested interaction
/// mode into the workflow. The requested interaction mode is represented by a
/// service specified by the `selector` field of [`RunSelector`]. When that
/// service terminates, this workflow will resume running the inspector service
/// until the user requests some other mouse interaction service to run.
///
/// In most cases downstream users will not need to call this function since the
/// [`SelectionPlugin`] will use this to build and run the default selection
/// workflow. If you are not using the [`SelectionPlugin`] that we provide and
/// want to customize the inspector service, then you could use this to build a
/// customized selection workflow by passingin a custom inspector service.
pub(crate) fn build_selection_workflow(
    default_service: Service<(), ()>,
    new_selector_service: Service<(), (), StreamOf<RunSelector>>,
) -> impl FnOnce(Scope<(), ()>, &mut Builder) -> DeliverySettings {
    move |scope, builder| {
        // This creates a service that will listen to run_service_buffer.
        // The job of this service is to atomically pull the most recent item
        // out of the buffer and also close the buffer gate to ensure that we
        // never have multiple selector services racing to be injected. If we
        // don't bother to close the gate after pulling exactly one selection
        // service, then it's theoretically possible for multiple selection
        // services to get simultaneously injected after the trim operation
        // finishes.
        let process_new_selector_service = builder
            .commands()
            .spawn_service(process_new_selector.into_blocking_service());

        // The run_service_buffer queues up the most recent RunSelector request
        // sent in by the user. That request will be held in this buffer while
        // we wait for any ongoing mouse interaction services to cleanly exit
        // after we trigger the trim operation.
        let run_service_buffer = builder.create_buffer::<RunSelector>(BufferSettings::keep_last(1));
        let input = scope.input.fork_clone(builder);
        // Run the default inspector service
        let inspector = input.clone_chain(builder).then_node(default_service);

        // Create a node that reads RunSelector events from the world and streams
        // them into the workflow.
        let new_selector_node = input.clone_chain(builder).then_node(new_selector_service);
        builder.connect(new_selector_node.output, scope.terminate);
        new_selector_node
            .streams
            .chain(builder)
            .inner()
            .connect(run_service_buffer.input_slot());

        let open_gate = builder.create_gate_open(run_service_buffer);
        // Create an operation that trims the gate opening operation, the injected
        // selector service, and the default inspector service.
        let trim = builder.create_trim([TrimBranch::between(open_gate.input, inspector.input)]);
        builder.connect(trim.output, open_gate.input);

        // Create a sequence where we listen for updates in the run service buffer,
        // then pull an item out of the buffer (if available), then begin the
        // trim of all ongoing selection services, then opening the gate of the
        // buffer to allow new selection services to be started.
        builder
            .listen(run_service_buffer)
            .then(process_new_selector_service)
            .dispose_on_none()
            .connect(trim.input);

        // After we open the gate it is safe to inject the user-requested selecion
        // service. Once that service finishes, we will trigger the inspector to
        // resume.
        open_gate
            .output
            .chain(builder)
            .map_block(|r: RunSelector| (r.input, r.selector))
            .then_injection()
            .trigger()
            .connect(inspector.input);

        // This workflow only makes sense to run in serial.
        DeliverySettings::Serial
    }
}

pub fn send_double_click_event(
    mut select: EventReader<Select>,
    mut double_clicked: Local<DoubleClickSelection>,
    mut double_click_select: EventWriter<DoubleClickSelect>,
) {
    for selected in select.read() {
        let current_time = Instant::now();

        let Some(selected_entity) = selected.0.map(|c| c.candidate) else {
            return;
        };

        if let Some(last_entity) = double_clicked.last_selected_entity {
            let elapsed_time = current_time
                .duration_since(double_clicked.last_selected_time)
                .as_millis();

            if last_entity == selected_entity && elapsed_time < DOUBLE_CLICK_DURATION_MILLISECONDS {
                double_click_select.write(DoubleClickSelect(selected_entity));
            }
        }
        double_clicked.last_selected_entity = Some(selected_entity);
        double_clicked.last_selected_time = current_time;
    }
}
