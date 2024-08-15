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

use bevy::prelude::*;
use bevy_impulse::*;

use crate::interaction::select::*;
use rmf_site_format::{Path, Floor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Resource)]
pub enum AnchorScope {
    Drawing,
    General,
    Site,
}

impl AnchorScope {
    pub fn is_site(&self) -> bool {
        match self {
            AnchorScope::Site => true,
            _ => false,
        }
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
    pub replace_side: Service<Option<Entity>, ()>,
    pub create_path: Service<Option<Entity>, ()>,
}

impl AnchorSelectionServices {
    pub fn from_app(
        helpers: &AnchorSelectionHelpers,
        app: &mut App,
    ) -> Self {
        let create_edges = spawn_create_edges_service(helpers, app);
        let replace_side = spawn_replace_side_service(helpers, app);
        let create_path = spawn_create_path_service(helpers, app);
        Self { create_edges, replace_side, create_path }
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

    pub fn create_floor(&mut self) {
        self.create_path::<Floor<Entity>>(
            create_path_with_texture::<Floor<Entity>>,
            3,
            false,
            true,
            AnchorScope::General,
        );
    }

    pub fn replace_side(
        &mut self,
        edge: Entity,
        side: Side,
        category: Category,
    ) -> bool {
        let scope = match category {
            Category::Lane | Category::Wall | Category::Door => AnchorScope::General,
            Category::Measurement => AnchorScope::Drawing,
            Category::Lift => AnchorScope::Site,
            _ => return false,
        };
        let entity = self.commands.spawn(SelectorInput(
            ReplaceSide::new(edge, side, scope)
        )).id();

        self.run.send(RunSelector {
            selector: self.services.replace_side,
            input: Some(entity),
        });

        true
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

    pub fn create_path<T: Bundle + From<Path<Entity>>>(
        &mut self,
        spawn_path: fn(Path<Entity>, &mut Commands) -> Entity,
        minimum_points: usize,
        allow_inner_loops: bool,
        implied_complete_loop: bool,
        scope: AnchorScope,
    ) {
        let entity = self.commands.spawn(SelectorInput(CreatePath::new(
            spawn_path, minimum_points, allow_inner_loops, implied_complete_loop, scope,
        ))).id();

        self.run.send(RunSelector {
            selector: self.services.create_path,
            input: Some(entity),
        });
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

pub fn exit_on_esc<T>(
    In((button, _)): In<(KeyCode, BufferKey<T>)>,
) -> SelectionNodeResult {
    if matches!(button, KeyCode::Escape) {
        // The escape key was pressed so we should exit this mode
        return Err(None);
    }

    Ok(())
}
