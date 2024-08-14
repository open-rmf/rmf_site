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
    Pose,
};
use bevy::{
    prelude::{*, Input},
    ecs::system::{SystemParam, StaticSystemParam}
};
use bevy_impulse::*;
use bevy_mod_raycast::deferred::RaycastMesh;
use std::{
    collections::HashSet,
    borrow::Borrow,
    error::Error,
};
use anyhow::{anyhow, Error as Anyhow};

pub mod create_edges;
use create_edges::*;

pub mod replace_edge_anchor;
use replace_edge_anchor::*;

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

        // let injection = open_gate.output.chain(builder)
        //     .map_block(|r: RunSelector| (r.input, r.selector))
        //     .then_injection_node();
        // injection.output.chain(builder)
        //     .trigger()
        //     .connect(inspector.input);

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
                    .then(hover_picking)
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

#[derive(Default)]
pub struct AnchorSelectionPlugin {}

impl Plugin for AnchorSelectionPlugin {
    fn build(&self, app: &mut App) {
        let helpers = AnchorSelectionHelpers::from_app(app);
        let services = AnchorSelectionServices::from_app(&helpers, app);
        app
            .insert_resource(AnchorScope::General)
            .insert_resource(helpers)
            .insert_resource(services);
    }
}

#[derive(Resource, Clone, Copy)]
pub struct AnchorSelectionHelpers {
    pub anchor_select_stream: Service<(), (), (Hover, Select)>,
    pub anchor_cursor_transform: Service<(), ()>,
    pub keyboard_just_pressed: Service<(), (), StreamOf<KeyCode>>,
    pub cleanup_anchor_selection: Service<(), ()>,
}

impl AnchorSelectionHelpers {
    pub fn from_app(app: &mut App) -> Self {
        let anchor_select_stream = app.spawn_selection_service::<AnchorFilter>();
        let anchor_cursor_transform = app.spawn_continuous_service(
            Update,
            select_anchor_cursor_transform
            .configure(|config: SystemConfigs|
                config.in_set(SelectionServiceStages::Pick)
            ),
        );
        let cleanup_anchor_selection = app.world.spawn_service(
            cleanup_anchor_selection.into_blocking_service()
        );

        let keyboard_just_pressed = app.world.resource::<KeyboardServices>()
            .keyboard_just_pressed;

        Self {
            anchor_select_stream,
            anchor_cursor_transform,
            keyboard_just_pressed,
            cleanup_anchor_selection,
        }
    }

    pub fn spawn_anchor_selection_workflow<State: 'static + Send + Sync>(
        &self,
        anchor_setup: Service<BufferKey<State>, SelectionNodeResult>,
        state_setup: Service<BufferKey<State>, SelectionNodeResult>,
        update_preview: Service<(Hover, BufferKey<State>), SelectionNodeResult>,
        update_current: Service<(SelectionCandidate, BufferKey<State>), SelectionNodeResult>,
        handle_key_code: Service<(KeyCode, BufferKey<State>), SelectionNodeResult>,
        cleanup_state: Service<BufferKey<State>, SelectionNodeResult>,
        world: &mut World,
    ) -> Service<Option<Entity>, ()> {
        world.spawn_io_workflow(build_anchor_selection_workflow(
            anchor_setup,
            state_setup,
            update_preview,
            update_current,
            handle_key_code,
            cleanup_state,
            self.anchor_cursor_transform,
            self.anchor_select_stream,
            self.keyboard_just_pressed,
            self.cleanup_anchor_selection
        ))
    }
}

#[derive(Resource, Clone, Copy)]
pub struct AnchorSelectionServices {
    pub create_edges: Service<Option<Entity>, ()>,
}

impl AnchorSelectionServices {
    pub fn from_app(
        helpers: &AnchorSelectionHelpers,
        app: &mut App,
    ) -> Self {
        let create_edges = spawn_create_edges_service(helpers, app);
        Self { create_edges }
    }
}

#[derive(SystemParam)]
pub struct AnchorSelection<'w, 's> {
    pub services: Res<'w, AnchorSelectionServices>,
    pub run: EventWriter<'w, RunSelector>,
    pub commands: Commands<'w, 's>,
}

impl<'w, 's> AnchorSelection<'w, 's> {
    pub fn create_lanes(&mut self) {
        self.create_edges::<Lane<Entity>>(
            EdgeContinuity::Continuous,
            AnchorScope::General,
        );
    }

    pub fn create_measurements(&mut self) {
        self.create_edges::<Measurement<Entity>>(
            EdgeContinuity::Separate,
            AnchorScope::Drawing,
        )
    }

    pub fn create_walls(&mut self) {
        self.create_edges::<Wall<Entity>>(
            EdgeContinuity::Continuous,
            AnchorScope::General,
        );
    }

    pub fn create_door(&mut self) {
        self.create_edges::<Door<Entity>>(
            EdgeContinuity::Single,
            AnchorScope::General,
        )
    }

    pub fn create_lift(&mut self) {
        self.create_edges::<LiftProperties<Entity>>(
            EdgeContinuity::Single,
            AnchorScope::Site,
        )
    }

    pub fn create_edges<T: Bundle + From<Edge<Entity>>>(
        &mut self,
        continuity: EdgeContinuity,
        scope: AnchorScope,
    ) {
        let entity = self.commands.spawn(SelectorInput(
            CreateEdges::new::<T>(continuity, scope)
        )).id();

        self.run.send(RunSelector {
            selector: self.services.create_edges,
            input: Some(entity),
        });
    }
}

pub type SelectionNodeResult = Result<(), Option<Anyhow>>;

pub trait CommonNodeErrors {
    type Value;
    fn or_broken_buffer(self) -> Result<Self::Value, Option<Anyhow>>;
    fn or_missing_state(self) -> Result<Self::Value, Option<Anyhow>>;
    fn or_broken_query(self) -> Result<Self::Value, Option<Anyhow>>;
}

impl<T, E: Error> CommonNodeErrors for Result<T, E> {
    type Value = T;
    fn or_broken_buffer(self) -> Result<Self::Value, Option<Anyhow>> {
        self.map_err(|err|
            Some(anyhow!(
                "The buffer in the workflow has been despawned: {err}"
            ))
        )
    }

    fn or_missing_state(self) -> Result<Self::Value, Option<Anyhow>> {
        self.map_err(|err|
            Some(anyhow!(
                "The state is missing from the workflow buffer: {err}"
            ))
        )
    }

    fn or_broken_query(self) -> Result<Self::Value, Option<Anyhow>> {
        self.map_err(|err|
            Some(anyhow!(
                "A query that should have worked failed: {err}"
            ))
        )
    }
}

impl<T> CommonNodeErrors for Option<T> {
    type Value = T;
    fn or_broken_buffer(self) -> Result<Self::Value, Option<Anyhow>> {
        self.ok_or_else(||
            Some(anyhow!("The buffer in the workflow has been despawned"))
        )
    }

    fn or_missing_state(self) -> Result<Self::Value, Option<Anyhow>> {
        self.ok_or_else(||
            Some(anyhow!("The state is missing from the workflow buffer"))
        )
    }

    fn or_broken_query(self) -> Result<Self::Value, Option<Anyhow>> {
        self.ok_or_else(||
            Some(anyhow!("A query that should have worked failed"))
        )
    }
}

/// The first five services should be customized for the State data. The services
/// that return NodeResult should return `Ok(())` if it is okay for the
/// workflow to continue as normal, and they should return `Err(None)` if it's
/// time for the workflow to terminate as normal. If the workflow needs to
/// terminate because of an error, return `Err(Some(_))`.
pub fn build_anchor_selection_workflow<State: 'static + Send + Sync>(
    anchor_setup: Service<BufferKey<State>, SelectionNodeResult>,
    state_setup: Service<BufferKey<State>, SelectionNodeResult>,
    update_preview: Service<(Hover, BufferKey<State>), SelectionNodeResult>,
    update_current: Service<(SelectionCandidate, BufferKey<State>), SelectionNodeResult>,
    handle_key_code: Service<(KeyCode, BufferKey<State>), SelectionNodeResult>,
    cleanup_state: Service<BufferKey<State>, SelectionNodeResult>,
    anchor_cursor_transform: Service<(), ()>,
    anchor_select_stream: Service<(), (), (Hover, Select)>,
    keyboard_just_pressed: Service<(), (), StreamOf<KeyCode>>,
    cleanup_anchor_selection: Service<(), ()>,
) -> impl FnOnce(Scope<Option<Entity>, ()>, &mut Builder) {
    move |scope, builder| {
        let buffer = builder.create_buffer::<State>(BufferSettings::keep_last(1));

        let setup_node = builder.create_buffer_access(buffer);
        let begin_input_services = setup_node.output.chain(builder)
            .map_block(|(_, key)| key)
            .then(anchor_setup)
            .branch_for_err(|err| err
                .map_block(print_if_err).connect(scope.terminate)
            )
            .with_access(buffer)
            .map_block(|(_, key)| key)
            .then(state_setup)
            .branch_for_err(|err| err
                .map_block(print_if_err).connect(scope.terminate)
            )
            .output()
            .fork_clone(builder);

        scope.input.chain(builder)
            .then(extract_selector_input.into_blocking_callback())
            // If the setup failed (returned None), then terminate right away.
            .branch_for_err(|chain: Chain<_>| chain.connect(scope.terminate))
            .fork_option(
                |chain: Chain<_>| chain.then_push(buffer).connect(setup_node.input),
                |chain: Chain<_>| chain.connect(setup_node.input),
            );

        begin_input_services.clone_chain(builder)
            .then(anchor_cursor_transform)
            .unused();

        let select = begin_input_services.clone_chain(builder).then_node(anchor_select_stream);
        select.streams.0.chain(builder)
            .with_access(buffer)
            .then(update_preview)
            .dispose_on_ok()
            .map_block(print_if_err)
            .connect(scope.terminate);

        select.streams.1.chain(builder)
            .map_block(|s| s.0)
            .dispose_on_none()
            .with_access(buffer)
            .then(update_current)
            .dispose_on_ok()
            .map_block(print_if_err)
            .connect(scope.terminate);

        let keyboard = begin_input_services.clone_chain(builder).then_node(keyboard_just_pressed);
        keyboard.streams.chain(builder)
            .inner()
            .with_access(buffer)
            .then(handle_key_code)
            .dispose_on_ok()
            .map_block(print_if_err)
            .connect(scope.terminate);

        builder.on_cleanup(buffer, move |scope, builder| {
            let state_node = builder.create_node(cleanup_state);
            let anchor_node = builder.create_node(cleanup_anchor_selection);

            builder.connect(scope.input, state_node.input);
            state_node.output.chain(builder)
                .fork_result(
                    |ok| ok.connect(anchor_node.input),
                    |err| err.map_block(print_if_err).connect(anchor_node.input),
                );

            builder.connect(anchor_node.output, scope.terminate);
        });
    }
}

fn print_if_err(err: Option<Anyhow>) {
    if let Some(err) = err {
        error!("{err}");
    }
}

pub fn cleanup_anchor_selection(
    In(_): In<()>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut hidden_anchors: ResMut<HiddenSelectAnchorEntities>,
    mut anchor_scope: ResMut<AnchorScope>,
    mut highlight: ResMut<HighlightAnchors>,
) {
    cursor.remove_mode(SELECT_ANCHOR_MODE_LABEL, &mut visibility);
    set_visibility(cursor.site_anchor_placement, &mut visibility, false);
    set_visibility(cursor.level_anchor_placement, &mut visibility, false);
    for e in hidden_anchors.drawing_anchors.drain() {
        set_visibility(e, &mut visibility, true);
    }

    highlight.0 = false;

    *anchor_scope = AnchorScope::General;
}

pub fn extract_selector_input<T: 'static + Send + Sync>(
    In(e): In<Option<Entity>>,
    world: &mut World,
) -> Result<Option<T>, ()> {
    let Some(e) = e else {
        // There is no input to provide, so move ahead with the workflow
        return Ok(None);
    };

    let Some(mut e_mut) = world.get_entity_mut(e) else {
        error!(
            "Could not begin selector service because the input entity {e:?} \
            does not exist.",
        );
        return Err(());
    };

    let Some(input) = e_mut.take::<SelectorInput<T>>() else {
        error!(
            "Could not begin selector service because the input entity {e:?} \
            did not contain a value {:?}. This is a bug, please report it.",
            std::any::type_name::<SelectorInput<T>>(),
        );
        return Err(());
    };

    e_mut.despawn_recursive();

    Ok(Some(input.0))
}

#[derive(SystemParam)]
pub struct AnchorFilter<'w, 's> {
    inspect: InspectorFilter<'w ,'s>,
    anchors: Query<'w, 's, (), With<Anchor>>,
    cursor: Res<'w, Cursor>,
    anchor_scope: Res<'w, AnchorScope>,
    workspace: Res<'w, CurrentWorkspace>,
    open_sites: Query<'w, 's, Entity, With<NameOfSite>>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
    commands: Commands<'w, 's>,
    current_drawing: Res<'w, CurrentEditDrawing>,
    drawings: Query<'w, 's, &'static PixelsPerMeter, With<DrawingMarker>>,
}

impl<'w, 's> SelectionFilter for AnchorFilter<'w ,'s> {
    fn filter_pick(&mut self, select: Entity) -> Option<Entity> {
        self.inspect.filter_pick(select)
            .and_then(|e| {
                if self.anchors.contains(e) {
                    Some(e)
                } else {
                    None
                }
            })
    }

    fn filter_select(&mut self, target: Entity) -> Option<Entity> {
        if self.anchors.contains(target) {
            Some(target)
        } else {
            None
        }
    }

    fn on_click(&mut self, hovered: Hover) -> Option<Select> {
        if let Some(candidate) = hovered.0 {
            return Some(Select::new(Some(candidate)));
        }

        // There was no anchor currently hovered which means we need to create
        // a new provisional anchor.
        let Ok(tf) = self.transforms.get(self.cursor.frame) else {
            error!("Cannot find cursor transform");
            return None;
        };

        let new_anchor = match self.anchor_scope.as_ref() {
            AnchorScope::Site => {
                let Some(site) = self.workspace.to_site(&self.open_sites) else {
                    error!("Cannot find current site");
                    return None;
                };
                let new_anchor = self.commands.spawn(AnchorBundle::at_transform(tf)).id();
                self.commands.entity(site).add_child(new_anchor);
                new_anchor
            }
            AnchorScope::Drawing => {
                let Some(current_drawing) = self.current_drawing.target() else {
                    error!(
                        "We are supposed to be in a drawing scope but there is \
                        no current drawing"
                    );
                    return None;
                };
                let drawing = current_drawing.drawing;
                let Ok(ppm) = self.drawings.get(drawing) else {
                    error!("Cannot find pixels per meter of current drawing");
                    return None;
                };
                let pose = compute_parent_inverse_pose(&tf, &self.transforms, drawing)?;
                let ppm = ppm.0;
                self.commands
                    .spawn(AnchorBundle::new([pose.trans[0], pose.trans[1]].into()))
                    .insert(Transform::from_scale(Vec3::new(ppm, ppm, 1.0)))
                    .set_parent(drawing)
                    .id()
            }
            AnchorScope::General => {
                // TODO(@mxgrey): Consider putting the anchor directly into the
                // current level instead of relying on orphan behavior
                self.commands.spawn(AnchorBundle::at_transform(tf)).id()
            }
        };

        Some(Select::provisional(new_anchor))
    }
}

fn compute_parent_inverse_pose(
    tf: &GlobalTransform,
    transforms: &Query<&GlobalTransform>,
    parent: Entity,
) -> Option<Pose> {
    let parent_tf = match transforms.get(parent) {
        Ok(tf) => tf,
        Err(err) => {
            error!("Failed in fetching parent transform: {err}");
            return None;
        }
    };

    let inv_tf = parent_tf.affine().inverse();
    let goal_tf = tf.affine();
    let mut pose = Pose::default();
    pose.rot = pose.rot.as_euler_extrinsic_xyz();
    Some(pose.align_with(&Transform::from_matrix((inv_tf * goal_tf).into())))
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

pub fn anchor_selection_setup<State: Borrow<AnchorScope>>(
    In(key): In<BufferKey<State>>,
    access: BufferAccess<State>,
    anchors: Query<Entity, With<Anchor>>,
    drawings: Query<(), With<DrawingMarker>>,
    parents: Query<&'static Parent>,
    mut visibility: Query<&'static mut Visibility>,
    mut hidden_anchors: ResMut<HiddenSelectAnchorEntities>,
    mut current_anchor_scope: ResMut<AnchorScope>,
    mut cursor: ResMut<Cursor>,
    mut highlight: ResMut<HighlightAnchors>,
) -> SelectionNodeResult
where
    State: 'static + Send + Sync,
{
    let access = access.get(&key).or_broken_buffer()?;
    let state = access.newest().or_missing_state()?;
    let scope: &AnchorScope = (&*state).borrow();
    match scope {
        AnchorScope::General | AnchorScope::Site => {
            // If we are working with normal level or site requests, hide all drawing anchors
            for e in anchors.iter().filter(|e| {
                parents
                .get(*e)
                .is_ok_and(|p| drawings.get(p.get()).is_ok())
            }) {
                set_visibility(e, &mut visibility, false);
                hidden_anchors.drawing_anchors.insert(e);
            }
        }
        // Nothing to hide, it's done by the drawing editor plugin
        AnchorScope::Drawing => {}
    }

    if scope.is_site() {
        set_visibility(cursor.site_anchor_placement, &mut visibility, true);
    } else {
        set_visibility(cursor.level_anchor_placement, &mut visibility, true);
    }

    highlight.0 = true;

    *current_anchor_scope = *scope;

    cursor.add_mode(SELECT_ANCHOR_MODE_LABEL, &mut visibility);

    Ok(())
}
