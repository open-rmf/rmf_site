/*
 * Copyright (C) 2022 Open Source Robotics Foundation
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

use crate::{interaction::*, site::Anchor};
use bevy::{
    prelude::{*, Input},
    ecs::system::{SystemParam, StaticSystemParam}
};
use bevy_impulse::*;
use bevy_mod_raycast::deferred::RaycastMesh;
use std::collections::HashSet;

#[derive(Default)]
pub struct SelectPlugin {}

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut App) {
        app
        .init_resource::<SelectionBlockers>()
        .init_resource::<Selection>()
        .init_resource::<Hovering>()
        .add_event::<Select>()
        .add_event::<Hover>()
        .add_plugins((
            InspectorServicePlugin::default(),
            AnchorSelectPlugin::default(),
        ));

        let inspector_service = app.world.resource::<InspectorService>().inspector_service;
        let new_selector_service = app.spawn_event_streaming_service::<RunSelector>(Update);
        let select_workflow = app.world.spawn_io_workflow(build_select_workflow(
            inspector_service,
            new_selector_service,
        ));

        // Get the selection workflow running
        app.world.command(|commands| {
            commands.request((), select_workflow).detach();
        });
    }
}

pub fn build_select_workflow(
    inspector_service: Service<(), ()>,
    new_selector_service: Service<(), (), StreamOf<RunSelector>>,
) -> impl FnOnce(Scope<(), ()>, &mut Builder) -> DeliverySettings {
    move |scope, builder| {
        let process_new_selector_service = builder
            .commands()
            .spawn_service(process_new_selector.into_blocking_service());

        let run_service_buffer = builder.create_buffer::<RunSelector>(BufferSettings::keep_last(1));
        let input = scope.input.fork_clone(builder);
        let inspector = input.clone_chain(builder).then_node(inspector_service);
        let new_selector_node = input.clone_chain(builder).then_node(new_selector_service);
        builder.connect(new_selector_node.output, scope.terminate);
        new_selector_node.streams.chain(builder)
            .inner()
            .connect(run_service_buffer.input_slot());

        let open_gate = builder.create_gate_open(run_service_buffer);
        let trim = builder.create_trim([
            TrimBranch::between(open_gate.input, inspector.input),
        ]);
        builder.connect(trim.output, open_gate.input);

        builder.listen(run_service_buffer)
            .then(process_new_selector_service)
            .dispose_on_none()
            .connect(trim.input);

        open_gate.output.chain(builder)
            .map_block(|r: RunSelector| ((), r.selector))
            .then_injection()
            .trigger()
            .connect(inspector.input);

        DeliverySettings::Serial
    }
}

fn process_new_selector(
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

#[derive(Debug, Clone, Copy, Event)]
pub struct RunSelector {
    /// The select workflow will run this service until it terminates and then
    /// revert back to the inspector selector.
    selector: Service<(), ()>,
}

/// This component is put on entities with meshes to mark them as items that can
/// be interacted with to
#[derive(Component, Clone, Copy, Debug)]
pub struct Selectable {
    /// Toggle whether this entity is selectable
    pub is_selectable: bool,
    /// What element of the site is being selected when this entity is clicked
    pub element: Entity,
}

impl Selectable {
    pub fn new(element: Entity) -> Self {
        Selectable {
            is_selectable: true,
            element,
        }
    }
}

#[derive(Component, Debug, PartialEq, Eq)]
pub struct Selected {
    /// This object has been selected
    pub is_selected: bool,
    /// Another object is selected but wants this entity to be highlighted
    pub support_selected: HashSet<Entity>,
}

impl Selected {
    pub fn cue(&self) -> bool {
        self.is_selected || !self.support_selected.is_empty()
    }
}

impl Default for Selected {
    fn default() -> Self {
        Self {
            is_selected: false,
            support_selected: Default::default(),
        }
    }
}

/// Component to track whether an element should be viewed in the Hovered state
/// for the selection tool.
#[derive(Component, Debug, PartialEq, Eq)]
pub struct Hovered {
    /// The cursor is hovering on this object specifically
    pub is_hovered: bool,
    /// The cursor is hovering on a different object which wants this entity
    /// to be highlighted.
    pub support_hovering: HashSet<Entity>,
}

impl Hovered {
    pub fn cue(&self) -> bool {
        self.is_hovered || !self.support_hovering.is_empty()
    }
}

impl Default for Hovered {
    fn default() -> Self {
        Self {
            is_hovered: false,
            support_hovering: Default::default(),
        }
    }
}

/// Used as a resource to keep track of which entity is currently selected.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Resource)]
pub struct Selection(pub Option<Entity>);

/// Used as a resource to keep track of which entity is currently hovered.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Resource)]
pub struct Hovering(pub Option<Entity>);

/// Used as an event to command a change in the selected entity.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Event, Stream)]
pub struct Select(pub Option<Entity>);

/// Used as an event to command a change in the hovered entity.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Event, Stream)]
pub struct Hover(pub Option<Entity>);

/// A resource to track what kind of blockers are preventing the selection
/// behavior from being active
#[derive(Resource)]
pub struct SelectionBlockers {
    /// An entity is being dragged
    pub dragging: bool,
    /// An entity is being placed
    pub placing: bool,
}

impl SelectionBlockers {
    pub fn blocking(&self) -> bool {
        self.dragging || self.placing
    }
}

impl Default for SelectionBlockers {
    fn default() -> Self {
        SelectionBlockers {
            dragging: false,
            placing: false,
        }
    }
}

pub fn make_selectable_entities_pickable(
    mut commands: Commands,
    new_selectables: Query<(Entity, &Selectable), Added<Selectable>>,
    targets: Query<(Option<&Hovered>, Option<&Selected>)>,
) {
    for (entity, selectable) in &new_selectables {
        commands
            .entity(entity)
            .insert(RaycastMesh::<SiteRaycastSet>::default());

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

/// This allows an [`App`] to spawn a service that can stream Hover and
/// Select events that are managed by a filter. This can only be used with
/// [`App`] because some of the internal services are continuous, so they need
/// to be added to the schedule.
pub trait SpawnSelectionServiceExt {
    fn spawn_selection_service<F: SystemParam + 'static>(
        &mut self,
    ) -> Service<(), (), (Hover, Select)>
    where
        for<'w, 's> F::Item<'w, 's>: SelectionFilter;
}

impl SpawnSelectionServiceExt for App {
    fn spawn_selection_service<F: SystemParam + 'static>(
        &mut self,
    ) -> Service<(), (), (Hover, Select)>
    where
        for<'w, 's> F::Item<'w, 's>: SelectionFilter,
    {
        let hover_picking = self.spawn_continuous_service(
            Update,
            hover_picking_service::<F>
            .configure(|config: SystemConfigs|
                config.in_set(SelectionServiceStages::Pick)
            ),
        );

        let hover_service = self.spawn_continuous_service(
            Update,
            hover_service::<F>
            .configure(|config: SystemConfigs|
                config
                .in_set(SelectionServiceStages::Hover)
                .after(SelectionServiceStages::Pick)
            ),
        );

        let select_service = self.spawn_continuous_service(
            Update,
            select_service::<F>
            .configure(|config: SystemConfigs|
                config
                .in_set(SelectionServiceStages::Select)
                .after(SelectionServiceStages::Hover)
            ),
        );

        self.world.spawn_workflow::<_, _, (Hover, Select), _>(|scope, builder| {
            let input_clone = scope.input.fork_clone(builder);
            input_clone.clone_chain(builder)
                .then(clear_picked.into_blocking_callback())
                .then(hover_picking)
                .connect(scope.terminate);

            let hover = builder.create_node(hover_service);
            builder.connect(hover.streams, scope.streams.0);
            builder.connect(hover.output, scope.terminate);

            let select = builder.create_node(select_service);
            builder.connect(select.streams, scope.streams.1);
            builder.connect(select.output, scope.terminate);

            // This is just a dummy buffer to let us have a cleanup workflow
            let buffer = builder.create_buffer::<()>(BufferSettings::keep_all());
            builder.on_cleanup(buffer, |scope, builder| {
                scope.input.chain(builder)
                    .trigger()
                    .then(clear_hover_select.into_blocking_callback())
                    .connect(scope.terminate);
            });
        })
    }
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SelectionServiceStages {
    Pick,
    Hover,
    Select,
}

#[derive(Resource)]
pub struct InspectorService {
    /// Workflow that updates the [`Selection`] as well as [`Hovered`] and
    /// [`Selected`] states in the application.
    pub inspector_service: Service<(), ()>,
    /// Workflow that outputs hover and select streams that are compatible with
    /// a general inspector. This service never terminates.
    pub inspector_select_service: Service<(), (), (Hover, Select)>,
    pub inspector_cursor_transform: Service<(), ()>,
}

#[derive(Default)]
pub struct InspectorServicePlugin {}

impl Plugin for InspectorServicePlugin {
    fn build(&self, app: &mut App) {
        let inspector_select_service = app.spawn_selection_service::<InspectorFilter>();
        let inspector_cursor_transform = app.spawn_continuous_service(
            Update, inspector_cursor_transform,
        );
        let selection_update = app.spawn_service(selection_update);

        let inspector_service = app.world.spawn_workflow(|scope, builder| {
            let fork_input = scope.input.fork_clone(builder);
            fork_input.clone_chain(builder).then(inspector_cursor_transform).unused();
            let selection = fork_input.clone_chain(builder).then_node(inspector_select_service);
            selection.streams.1.chain(builder).then(selection_update).unused();
            builder.connect(selection.output, scope.terminate);
        });

        app.world.insert_resource(InspectorService {
            inspector_service,
            inspector_select_service,
            inspector_cursor_transform,
        });
    }
}

#[derive(Resource)]
pub struct AnchorSelectService {
    pub anchor_select_service: Service<(), (), (Hover, Select)>,
}

#[derive(Default)]
pub struct AnchorSelectPlugin {}

impl Plugin for AnchorSelectPlugin {
    fn build(&self, app: &mut App) {
        let anchor_select_service = app.spawn_selection_service::<AnchorFilter>();
        app.insert_resource(AnchorSelectService { anchor_select_service });
    }
}

#[derive(SystemParam)]
pub struct AnchorFilter<'w, 's> {
    inspect: InspectorFilter<'w ,'s>,
    anchors: Query<'w, 's, (), With<Anchor>>,
}

impl<'w, 's> SelectionFilter for AnchorFilter<'w ,'s> {
    fn apply_filter(&mut self, select: Entity) -> Option<Entity> {
        self.inspect.apply_filter(select)
            .and_then(|e| {
                if self.anchors.contains(e) {
                    Some(e)
                } else {
                    None
                }
            })
    }
}

pub trait SelectionFilter: SystemParam {
    fn apply_filter(&mut self, select: Entity) -> Option<Entity>;
}

#[derive(SystemParam)]
pub struct InspectorFilter<'w, 's> {
    selectables: Query<'w, 's, &'static Selectable>,
}

impl<'w, 's> SelectionFilter for InspectorFilter<'w, 's> {
    fn apply_filter(&mut self, select: Entity) -> Option<Entity> {
        self.selectables.get(select).ok().map(|selectable| selectable.element)
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
pub fn hover_picking_service<Filter: SystemParam + 'static>(
    In(ContinuousService { key }): ContinuousServiceInput<(), ()>,
    requests: ContinuousQuery<(), ()>,
    mut picks: EventReader<ChangePick>,
    mut hover: EventWriter<Hover>,
    filter: StaticSystemParam<Filter>,
)
where
    for<'w, 's> Filter::Item<'w, 's>: SelectionFilter,
{
    let Some(requests) = requests.view(&key) else {
        return;
    };

    if requests.is_empty() {
        // Nothing is asking for this service to run, so skip it
        return;
    }

    let mut filter = filter.into_inner();

    if let Some(pick) = picks.read().last() {
        hover.send(Hover(
            pick.to.and_then(|change_pick_to| filter.apply_filter(change_pick_to))
        ));
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
    mut requests: ContinuousQuery<(), (), Hover>,
    mut hovered: Query<&mut Hovered>,
    mut hovering: ResMut<Hovering>,
    mut hover: EventReader<Hover>,
    mouse_button_input: Res<Input<MouseButton>>,
    touch_input: Res<Touches>,
    mut select: EventWriter<Select>,
    blockers: Option<Res<PickingBlockers>>,
    filter: StaticSystemParam<Filter>,
)
where
    for<'w, 's> Filter::Item<'w, 's>: SelectionFilter,
{
    let Some(mut requests) = requests.get_mut(&key) else {
        return;
    };

    if requests.is_empty() {
        // Nothing is asking for this service to run
        return;
    }

    let mut filter = filter.into_inner();

    if let Some(new_hovered) = hover.read().last() {
        let new_hovered = new_hovered.0.and_then(|e| filter.apply_filter(e));
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
            requests.for_each(|order| order.streams().send(Hover(new_hovered)));
        }
    }

    let clicked = mouse_button_input.just_pressed(MouseButton::Left)
        || touch_input.iter_just_pressed().next().is_some();
    let blocked = blockers.filter(|x| x.blocking()).is_some();

    if clicked && !blocked {
        if let Some(current_hovered) = hovering.0 {
            select.send(Select(Some(current_hovered)));
        }
    }
}

/// A continuous service that filters [`Select`] events and issues out a
/// [`Hover`] stream.
///
/// This complements [`hover_service`] and [`hover_picking`]
/// and is the final piece of the [`SelectionService`] workflow.
pub fn select_service<Filter: SystemParam + 'static>(
    In(ContinuousService{ key }): ContinuousServiceInput<(), (), Select>,
    mut requests: ContinuousQuery<(), (), Select>,
    mut select: EventReader<Select>,
    filter: StaticSystemParam<Filter>,
)
where
    for<'w, 's> Filter::Item<'w, 's>: SelectionFilter,
{
    let Some(mut requests) = requests.get_mut(&key) else {
        return;
    };

    if requests.is_empty() {
        // Nothing is asking for this service to run
        return;
    }

    let mut filter = filter.into_inner();

    for selected in select.read().map(|s| s.0.and_then(|e| filter.apply_filter(e))) {
        requests.for_each(|order| order.streams().send(Select(selected)));
    }
}

pub fn selection_update(
    In(BlockingService { request: Select(new_selection), .. }): BlockingServiceInput<Select>,
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
) {
    if selection.0 != new_selection {
        if let Some(previous_selection) = selection.0 {
            if let Ok(mut selected) = selected.get_mut(previous_selection) {
                selected.is_selected = false;
            }
        }

        if let Some(new_selection) = new_selection {
            if let Ok(mut selected) = selected.get_mut(new_selection) {
                selected.is_selected = true;
            }
        }

        selection.0 = new_selection;
    }
}

/// This is used to clear out the currently picked item at the start of a new
/// selection workflow to make sure the Hover events don't get lost during the
/// workflow switch.
pub fn clear_picked(
    In(_): In<()>,
    mut picked: ResMut<Picked>,
) {
    if picked.0.is_some() {
        picked.0 = None;
    }
}

/// This is used to clear out hoverings and selections from a workflow that is
/// cleaning up so that these properties don't spill over into other workflows.
pub fn clear_hover_select(
    In(_): In<()>,
    mut hovered: Query<&mut Hovered>,
    mut hovering: ResMut<Hovering>,
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
) {
    if let Some(previous_hovering) = hovering.0.take() {
        if let Ok(mut hovered) = hovered.get_mut(previous_hovering) {
            hovered.is_hovered = false;
        }
    }

    if let Some(previous_selection) = selection.0.take() {
        if let Ok(mut selected) = selected.get_mut(previous_selection) {
            selected.is_selected = false;
        }
    }
}

pub fn handle_selection_picking(
    blockers: Option<Res<SelectionBlockers>>,
    mode: Res<InteractionMode>,
    selectables: Query<&Selectable>,
    anchors: Query<(), With<Anchor>>,
    mut picks: EventReader<ChangePick>,
    mut hover: EventWriter<Hover>,
) {
    if let Some(blockers) = blockers {
        if blockers.blocking() {
            hover.send(Hover(None));
            return;
        }
    }

    if !mode.is_selecting() {
        hover.send(Hover(None));
        return;
    }

    for pick in picks.read() {
        hover.send(Hover(
            pick.to
                .and_then(|change_pick_to| {
                    selectables
                        .get(change_pick_to)
                        .ok()
                        .map(|selectable| selectable.element)
                })
                .and_then(|change_pick_to| {
                    if let InteractionMode::SelectAnchor(_) = *mode {
                        if anchors.contains(change_pick_to) {
                            Some(change_pick_to)
                        } else {
                            None
                        }
                    } else {
                        Some(change_pick_to)
                    }
                }),
        ));
    }
}

pub fn maintain_hovered_entities(
    mut hovered: Query<&mut Hovered>,
    mut hovering: ResMut<Hovering>,
    mut hover: EventReader<Hover>,
    mouse_button_input: Res<Input<MouseButton>>,
    touch_input: Res<Touches>,
    mut select: EventWriter<Select>,
    mode: Res<InteractionMode>,
    blockers: Option<Res<PickingBlockers>>,
) {
    if let Some(new_hovered) = hover.read().last() {
        if hovering.0 != new_hovered.0 {
            if let Some(previous_hovered) = hovering.0 {
                if let Ok(mut hovering) = hovered.get_mut(previous_hovered) {
                    hovering.is_hovered = false;
                }
            }

            if let Some(new_hovered) = new_hovered.0 {
                if let Ok(mut hovering) = hovered.get_mut(new_hovered) {
                    hovering.is_hovered = true;
                }
            }

            hovering.0 = new_hovered.0;
        }
    }

    let clicked = mouse_button_input.just_pressed(MouseButton::Left)
        || touch_input.iter_just_pressed().next().is_some();
    let blocked = blockers.filter(|x| x.blocking()).is_some();

    if clicked && !blocked {
        if let Some(current_hovered) = hovering.0 {
            // TODO(luca) refactor to remove this hack
            // Skip if we are in SelectAnchor3D mode
            if let InteractionMode::SelectAnchor3D(_) = &*mode {
                return;
            }
            select.send(Select(Some(current_hovered)));
        }
    }
}

pub fn maintain_selected_entities(
    mode: Res<InteractionMode>,
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
    mut select: EventReader<Select>,
) {
    if !mode.is_inspecting() {
        // We only maintain the "selected" entity when we are in Inspect mode.
        // Other "selecting" modes, like SelectAnchor, take in the selection as
        // an event and do not change the current selection that is being
        // inspected.
        return;
    }

    if let Some(new_selection) = select.read().last() {
        if selection.0 != new_selection.0 {
            if let Some(previous_selection) = selection.0 {
                if let Ok(mut selected) = selected.get_mut(previous_selection) {
                    selected.is_selected = false;
                }
            }

            if let Some(new_selection) = new_selection.0 {
                if let Ok(mut selected) = selected.get_mut(new_selection) {
                    selected.is_selected = true;
                }
            }

            selection.0 = new_selection.0;
        }
    }
}
