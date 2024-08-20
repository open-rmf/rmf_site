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

use crate::{
    CurrentWorkspace,
    keyboard::KeyboardServices,
    interaction::*,
    site::{drawing_editor::CurrentEditDrawing, Anchor, AnchorBundle, DrawingMarker},
};
use rmf_site_format::{
    Door, Edge, Lane, LiftProperties, Measurement, NameOfSite, PixelsPerMeter, Wall,
    Pose, Side, Category,
};
use bevy::{
    prelude::{*, Input},
    ecs::system::{SystemParam, StaticSystemParam}
};
use bevy_impulse::*;
use bevy_mod_raycast::{
    deferred::{RaycastMesh, RaycastSource},
    primitives::rays::Ray3d,
};
use std::{
    collections::HashSet,
    borrow::Borrow,
    error::Error,
};
use anyhow::{anyhow, Error as Anyhow};

pub mod create_edges;
use create_edges::*;

pub mod create_path;
use create_path::*;

pub mod create_point;
use create_point::*;

pub mod place_object_3d;
pub use place_object_3d::*;

pub mod replace_point;
use replace_point::*;

pub mod replace_side;
use replace_side::*;

pub mod select_anchor;
pub use select_anchor::*;

pub const SELECT_ANCHOR_MODE_LABEL: &'static str = "select_anchor";

#[derive(Default)]
pub struct SelectPlugin {}

impl Plugin for SelectPlugin {
    fn build(&self, app: &mut App) {
        app
        .configure_sets(
            Update,
            (
                SelectionServiceStages::Pick,
                SelectionServiceStages::PickFlush,
                SelectionServiceStages::Hover,
                SelectionServiceStages::HoverFlush,
                SelectionServiceStages::Select,
                SelectionServiceStages::SelectFlush,
            ).chain()
        )
        .init_resource::<SelectionBlockers>()
        .init_resource::<Selection>()
        .init_resource::<Hovering>()
        .add_event::<Select>()
        .add_event::<Hover>()
        .add_event::<RunSelector>()
        .add_systems(
            Update,
            (
                (apply_deferred, flush_impulses())
                .chain()
                .in_set(SelectionServiceStages::PickFlush),
                (apply_deferred, flush_impulses())
                .chain()
                .in_set(SelectionServiceStages::HoverFlush),
                (apply_deferred, flush_impulses())
                .chain()
                .in_set(SelectionServiceStages::SelectFlush),
            )
        )
        .add_plugins((
            InspectorServicePlugin::default(),
            AnchorSelectionPlugin::default(),
            ObjectPlacementPlugin::default(),
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
            .map_block(|r: RunSelector| (r.input, r.selector))
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
    selector: Service<Option<Entity>, ()>,
    /// If there is input for the selector, it will be stored in a [`SelectorInput`]
    /// component in this entity. The entity will be despawned as soon as the
    /// input is extracted.
    input: Option<Entity>,
}

#[derive(Component)]
pub struct SelectorInput<T>(T);

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
pub struct Select(pub Option<SelectionCandidate>);

impl Select {
    pub fn new(candidate: Option<Entity>) -> Select {
        Select(candidate.map(|c| SelectionCandidate::new(c)))
    }

    pub fn provisional(candidate: Entity) -> Select {
        Select(Some(SelectionCandidate::provisional(candidate)))
    }
}

#[derive(Debug, Clone, Copy)]
pub struct SelectionCandidate {
    /// The entity that's being requested as a selection
    pub candidate: Entity,
    /// The entity was created specifically to be selected, so if it ends up
    /// going unused by the workflow then it should be despawned.
    pub provisional: bool,
}

impl SelectionCandidate {
    pub fn new(candidate: Entity) -> SelectionCandidate {
        SelectionCandidate { candidate, provisional: false }
    }

    pub fn provisional(candidate: Entity) -> SelectionCandidate {
        SelectionCandidate { candidate, provisional: true }
    }
}

/// Used as an event to command a change in the hovered entity.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Event, Stream)]
pub struct Hover(pub Option<Entity>);

/// A resource to track what kind of blockers are preventing the selection
/// behavior from being active
#[derive(Resource)]
pub struct SelectionBlockers {
    /// An entity is being dragged
    pub dragging: bool,
}

impl SelectionBlockers {
    pub fn blocking(&self) -> bool {
        self.dragging
    }
}

impl Default for SelectionBlockers {
    fn default() -> Self {
        SelectionBlockers {
            dragging: false,
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
        let picking_service = self.spawn_continuous_service(
            Update,
            picking_service::<F>
            .configure(|config: SystemConfigs|
                config.in_set(SelectionServiceStages::Pick)
            ),
        );

        let hover_service = self.spawn_continuous_service(
            Update,
            hover_service::<F>
            .configure(|config: SystemConfigs|
                config.in_set(SelectionServiceStages::Hover)
            ),
        );

        let select_service = self.spawn_continuous_service(
            Update,
            select_service::<F>
            .configure(|config: SystemConfigs|
                config.in_set(SelectionServiceStages::Select)
            ),
        );

        self.world.spawn_workflow::<_, _, (Hover, Select), _>(|scope, builder| {
            let hover = builder.create_node(hover_service);
            builder.connect(hover.streams, scope.streams.0);
            builder.connect(hover.output, scope.terminate);

            let select = builder.create_node(select_service);
            builder.connect(select.streams, scope.streams.1);
            builder.connect(select.output, scope.terminate);

            // Activate all the services at the start
            scope.input.chain(builder).fork_clone((
                |chain: Chain<_>| chain
                    .then(refresh_picked.into_blocking_callback())
                    .then(picking_service)
                    .connect(scope.terminate),
                |chain: Chain<_>| chain.connect(hover.input),
                |chain: Chain<_>| chain.connect(select.input),
            ));

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

// TODO(@mxgrey): Remove flush stages when we move to bevy 0.13 which can infer
// when to flush
#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SelectionServiceStages {
    Pick,
    PickFlush,
    Hover,
    HoverFlush,
    Select,
    SelectFlush,
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
            Update,
            inspector_cursor_transform
            .configure(|config: SystemConfigs|
                config.in_set(SelectionServiceStages::Pick)
            ),
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

pub type SelectionNodeResult = Result<(), Option<Anyhow>>;

pub trait CommonNodeErrors {
    type Value;
    fn or_broken_buffer(self) -> Result<Self::Value, Option<Anyhow>>;
    fn or_broken_state(self) -> Result<Self::Value, Option<Anyhow>>;
    fn or_broken_query(self) -> Result<Self::Value, Option<Anyhow>>;
}

impl<T, E: Error> CommonNodeErrors for Result<T, E> {
    type Value = T;
    fn or_broken_buffer(self) -> Result<Self::Value, Option<Anyhow>> {
        self.map_err(|err| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!(
                "The buffer in the workflow is broken: {err}. Backtrace:\n{backtrace}"
            ))
        })
    }

    fn or_broken_state(self) -> Result<Self::Value, Option<Anyhow>> {
        self.map_err(|err| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!(
                "The state is missing from the workflow buffer: {err}. Backtrace:\n{backtrace}"
            ))
        })
    }

    fn or_broken_query(self) -> Result<Self::Value, Option<Anyhow>> {
        self.map_err(|err| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!(
                "A query that should have worked failed: {err}. Backtrace:\n{backtrace}"
            ))
        })
    }
}

impl<T> CommonNodeErrors for Option<T> {
    type Value = T;
    fn or_broken_buffer(self) -> Result<Self::Value, Option<Anyhow>> {
        self.ok_or_else(|| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!("The buffer in the workflow has been despawned. Backtrace:\n{backtrace}"))
        })
    }

    fn or_broken_state(self) -> Result<Self::Value, Option<Anyhow>> {
        self.ok_or_else(|| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!("The state is missing from the workflow buffer. Backtrace:\n{backtrace}"))
        })
    }

    fn or_broken_query(self) -> Result<Self::Value, Option<Anyhow>> {
        self.ok_or_else(|| {
            let backtrace = std::backtrace::Backtrace::force_capture();
            Some(anyhow!("A query that should have worked failed. Backtrace:\n{backtrace}"))
        })
    }
}

pub trait SelectionFilter: SystemParam {
    /// If the target entity is being picked, give back the entity that should
    /// be recognized as the hovered entity. Return [`None`] to behave as if
    /// nothing is being hovered.
    fn filter_pick(&mut self, target: Entity) -> Option<Entity>;

    /// If the target entity is being hovered or selected, give back the entity
    /// that should be recognized as the hovered or selected entity. Return
    /// [`None`] to deselect anything that might currently be selected.
    fn filter_select(&mut self, target: Entity) -> Option<Entity>;

    /// For the given hover state, indicate what kind of [`Select`] signal should
    /// be sent when the user clicks.
    fn on_click(&mut self, hovered: Hover) -> Option<Select>;
}

#[derive(SystemParam)]
pub struct InspectorFilter<'w, 's> {
    selectables: Query<'w, 's, &'static Selectable>,
}

impl<'w, 's> SelectionFilter for InspectorFilter<'w, 's> {
    fn filter_pick(&mut self, select: Entity) -> Option<Entity> {
        self.selectables.get(select).ok().map(|selectable| selectable.element)
    }
    fn filter_select(&mut self, target: Entity) -> Option<Entity> {
        Some(target)
    }
    fn on_click(&mut self, hovered: Hover) -> Option<Select> {
        Some(Select::new(hovered.0))
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
)
where
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
        hover.send(Hover(
            pick.to.and_then(|change_pick_to| filter.filter_pick(change_pick_to))
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
    mut orders: ContinuousQuery<(), (), Hover>,
    mut hovered: Query<&mut Hovered>,
    mut hovering: ResMut<Hovering>,
    mut hover: EventReader<Hover>,
    mouse_button_input: Res<Input<MouseButton>>,
    touch_input: Res<Touches>,
    mut select: EventWriter<Select>,
    blockers: Option<Res<PickingBlockers>>,
    filter: StaticSystemParam<Filter>,
    selection_blockers: Res<SelectionBlockers>,
)
where
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
    let blocked = blockers.filter(|x| x.blocking()).is_some();

    if clicked && !blocked {
        if let Some(new_select) = filter.on_click(Hover(hovering.0)) {
            select.send(new_select);
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
    mut orders: ContinuousQuery<(), (), Select>,
    mut select: EventReader<Select>,
    filter: StaticSystemParam<Filter>,
    mut commands: Commands,
)
where
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
                        if let Some(entity_mut) = commands.get_entity(selected.candidate) {
                            entity_mut.despawn_recursive();
                        }
                    }
                    continue;
                }
            }
        }

        orders.for_each(|order| order.streams().send(selected));
    }
}

pub fn selection_update(
    In(BlockingService { request: Select(new_selection), .. }): BlockingServiceInput<Select>,
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
) {
    if selection.0 != new_selection.map(|s| s.candidate) {
        if let Some(previous_selection) = selection.0 {
            if let Ok(mut selected) = selected.get_mut(previous_selection) {
                selected.is_selected = false;
            }
        }

        if let Some(new_selection) = new_selection {
            if let Ok(mut selected) = selected.get_mut(new_selection.candidate) {
                selected.is_selected = true;
            }
        }

        selection.0 = new_selection.map(|s| s.candidate);
    }
}

/// This is used to clear out the currently picked item at the start of a new
/// selection workflow to make sure the Hover events don't get lost during the
/// workflow switch.
pub fn refresh_picked(
    In(_): In<()>,
    mut picked: ResMut<Picked>,
) {
    picked.refresh = true;
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

/// Update the virtual cursor (dagger and circle) transform while in inspector mode
pub fn inspector_cursor_transform(
    In(ContinuousService { key }): ContinuousServiceInput<(), ()>,
    orders: ContinuousQuery<(), ()>,
    cursor: Res<Cursor>,
    raycast_sources: Query<&RaycastSource<SiteRaycastSet>>,
    mut transforms: Query<&mut Transform>,
) {
    let Some(orders) = orders.view(&key) else {
        return;
    };

    if orders.is_empty() {
        return;
    }

    let Ok(source) = raycast_sources.get_single() else {
        return;
    };
    let intersection = match source.get_nearest_intersection() {
        Some((_, intersection)) => intersection,
        None => {
            return;
        }
    };

    let mut transform = match transforms.get_mut(cursor.frame) {
        Ok(transform) => transform,
        Err(_) => {
            return;
        }
    };

    let ray = Ray3d::new(intersection.position(), intersection.normal());
    *transform = Transform::from_matrix(ray.to_aligned_transform([0., 0., 1.].into()));
}
