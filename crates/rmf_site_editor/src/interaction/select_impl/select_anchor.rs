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

use std::borrow::Borrow;
use std::collections::HashSet;

use bevy::ecs::{hierarchy::ChildOf, schedule::ScheduleConfigs, system::ScheduleSystem};
use bevy::prelude::*;
use crossflow::*;

use crate::interaction::{
    set_visibility, Cursor, GizmoBlockers, HighlightAnchors, IntersectGroundPlaneParams,
};
use crate::site::{AnchorBundle, ChildCabinAnchorGroup, CurrentEditDrawing, DrawingMarker};
use crate::workspace::CurrentWorkspace;
use crate::{interaction::select_impl::*, site::CurrentLevel};
use rmf_site_format::*;
use rmf_site_format::{Fiducial, Floor, LevelElevation, Location, Path, Point};
use rmf_site_picking::*;

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
        app.init_resource::<HiddenSelectAnchorEntities>()
            .insert_resource(AnchorScope::General)
            .insert_resource(helpers)
            .insert_resource(services);
    }
}

#[derive(Resource, Clone, Copy)]
pub struct AnchorSelectionHelpers {
    pub anchor_select_stream: Service<(), (), SelectionStreams>,
    pub anchor_cursor_transform: Service<(), ()>,
    pub keyboard_pressed: Service<(), (), StreamOf<(KeyCode, ButtonInputType)>>,
    pub cleanup_anchor_selection: Service<(), ()>,
}

impl AnchorSelectionHelpers {
    pub fn from_app(app: &mut App) -> Self {
        let anchor_select_stream = app.spawn_selection_service::<AnchorFilter>();
        let anchor_cursor_transform = app.spawn_continuous_service(
            Update,
            select_anchor_cursor_transform.configure(|config: ScheduleConfigs<ScheduleSystem>| {
                config.in_set(SelectionServiceStages::Pick)
            }),
        );
        let cleanup_anchor_selection = app
            .world_mut()
            .spawn_service(cleanup_anchor_selection.into_blocking_service());

        let keyboard_pressed = app.world().resource::<KeyboardServices>().keyboard_pressed;

        Self {
            anchor_select_stream,
            anchor_cursor_transform,
            keyboard_pressed,
            cleanup_anchor_selection,
        }
    }

    pub fn spawn_anchor_selection_workflow<State: 'static + Send + Sync>(
        &self,
        anchor_setup: Service<BufferKey<State>, SelectionNodeResult>,
        state_setup: Service<BufferKey<State>, SelectionNodeResult>,
        update_preview: Service<(Hover, BufferKey<State>), SelectionNodeResult>,
        update_current: Service<(SelectionCandidate, BufferKey<State>), SelectionNodeResult>,
        handle_key_code: Service<
            ((KeyCode, ButtonInputType), BufferKey<State>),
            SelectionNodeResult,
        >,
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
            self.keyboard_pressed,
            self.cleanup_anchor_selection,
        ))
    }
}

#[derive(Resource, Clone, Copy)]
pub struct AnchorSelectionServices {
    pub create_edges: Service<Option<Entity>, ()>,
    pub replace_side: Service<Option<Entity>, ()>,
    pub create_path: Service<Option<Entity>, ()>,
    pub create_point: Service<Option<Entity>, ()>,
    pub replace_point: Service<Option<Entity>, ()>,
}

impl AnchorSelectionServices {
    pub fn from_app(helpers: &AnchorSelectionHelpers, app: &mut App) -> Self {
        let create_edges = spawn_create_edges_service(helpers, app);
        let replace_side = spawn_replace_side_service(helpers, app);
        let create_path = spawn_create_path_service(helpers, app);
        let create_point = spawn_create_point_service(helpers, app);
        let replace_point = spawn_replace_point_service(helpers, app);
        Self {
            create_edges,
            replace_side,
            create_path,
            create_point,
            replace_point,
        }
    }
}

#[derive(SystemParam)]
pub struct AnchorSelection<'w, 's> {
    pub services: Res<'w, AnchorSelectionServices>,
    pub commands: Commands<'w, 's>,
}

impl<'w, 's> AnchorSelection<'w, 's> {
    pub fn create_lanes(&mut self) {
        self.create_edges::<Lane<Entity>>(EdgeCreationContinuity::Continuous, AnchorScope::General);
    }

    pub fn create_measurements(&mut self) {
        self.create_edges::<Measurement<Entity>>(
            EdgeCreationContinuity::Separate,
            AnchorScope::Drawing,
        )
    }

    pub fn create_walls(&mut self) {
        self.create_edges_with_texture::<Wall<Entity>>(
            EdgeCreationContinuity::Continuous,
            AnchorScope::General,
        );
    }

    pub fn create_door(&mut self) {
        self.create_edges_custom(
            CreateEdges::new::<Door<Entity>>(
                EdgeCreationContinuity::Separate,
                AnchorScope::General,
            )
            .with_finish(|edge, entity_mut| {
                let mut door: Door<Entity> = edge.into();
                door.kind.set_open();
                entity_mut.insert(door);
            }),
        );
    }

    pub fn create_lift(&mut self) {
        self.create_edges::<LiftProperties<Entity>>(
            EdgeCreationContinuity::Separate,
            AnchorScope::Site,
        )
    }

    pub fn create_floor(&mut self) {
        self.create_path(
            insert_path_with_texture::<Floor<Entity>>,
            3,
            false,
            true,
            AnchorScope::General,
        );
    }

    pub fn create_location(&mut self) {
        self.create_point::<Location<Entity>>(true, AnchorScope::General);
    }

    pub fn create_site_fiducial(&mut self) {
        self.create_point::<Fiducial<Entity>>(true, AnchorScope::Site);
    }

    pub fn create_drawing_fiducial(&mut self) {
        self.create_point::<Fiducial<Entity>>(true, AnchorScope::Drawing);
    }

    pub fn create_edges<T: Bundle + From<Edge<Entity>>>(
        &mut self,
        continuity: EdgeCreationContinuity,
        scope: AnchorScope,
    ) {
        self.create_edges_custom(CreateEdges::new::<T>(continuity, scope))
    }

    pub fn create_edges_with_texture<T: Bundle + From<Edge<Entity>>>(
        &mut self,
        continuity: EdgeCreationContinuity,
        scope: AnchorScope,
    ) {
        self.create_edges_custom(CreateEdges::new_with_texture::<T>(continuity, scope))
    }

    pub fn create_edges_custom(&mut self, creation: CreateEdges) {
        let state = self.commands.spawn(SelectorInput(creation)).id();
        self.send(RunSelector {
            selector: self.services.create_edges,
            input: Some(state),
        });
    }

    pub fn replace_side(&mut self, edge: Entity, side: Side, category: Category) -> bool {
        let scope = match category {
            Category::Lane | Category::Wall | Category::Door => AnchorScope::General,
            Category::Measurement => AnchorScope::Drawing,
            Category::Lift => AnchorScope::Site,
            _ => return false,
        };
        let state = self
            .commands
            .spawn(SelectorInput(ReplaceSide::new(edge, side, scope)))
            .id();

        self.send(RunSelector {
            selector: self.services.replace_side,
            input: Some(state),
        });

        true
    }

    pub fn create_path(
        &mut self,
        insert_path: fn(Path<Entity>, &mut EntityCommands) -> SelectionNodeResult,
        minimum_points: usize,
        allow_inner_loops: bool,
        implied_complete_loop: bool,
        scope: AnchorScope,
    ) {
        let state = self
            .commands
            .spawn(SelectorInput(CreatePath::new(
                insert_path,
                minimum_points,
                allow_inner_loops,
                implied_complete_loop,
                scope,
            )))
            .id();

        self.send(RunSelector {
            selector: self.services.create_path,
            input: Some(state),
        });
    }

    pub fn create_point<T: Bundle + From<Point<Entity>>>(
        &mut self,
        repeating: bool,
        scope: AnchorScope,
    ) {
        let state = self
            .commands
            .spawn(SelectorInput(CreatePoint::new::<T>(repeating, scope)))
            .id();

        self.send(RunSelector {
            selector: self.services.create_point,
            input: Some(state),
        });
    }

    pub fn replace_point(&mut self, point: Entity, scope: AnchorScope) {
        let state = self
            .commands
            .spawn(SelectorInput(ReplacePoint::new(point, scope)))
            .id();

        self.send(RunSelector {
            selector: self.services.replace_point,
            input: Some(state),
        });
    }

    fn send(&mut self, run: RunSelector) {
        self.commands.queue(move |world: &mut World| {
            world.send_event(run);
        });
    }
}

#[derive(Resource, Default)]
pub struct HiddenSelectAnchorEntities {
    /// All drawing anchors, hidden when users draw level entities such as walls, lanes, floors to
    /// make sure they don't connect to drawing anchors
    pub drawing_anchors: HashSet<Entity>,
}

/// The first five services should be customized for the State data. The services
/// that return [`SelectionNodeResult`] should return `Ok(())` if it is okay for the
/// workflow to continue as normal, and they should return `Err(None)` if it's
/// time for the workflow to terminate as normal. If the workflow needs to
/// terminate because of an error, return `Err(Some(_))`.
///
/// In most cases you should use [`AnchorSelectionHelpers::spawn_anchor_selection_workflow`]
/// instead of running this function yourself directly, unless you know that you
/// need to customize the last four services.
///
/// * `anchor_setup`: This is run once at the start of the workflow to prepare the
///   world to select anchors from the right kind of scope for the request. This
///   is usually just [`anchor_selection_setup`] instantiated for the right type
///   of state.
/// * `state_setup`: This is for any additional custom setup that is relevant to
///   the state information for your selection workflow. This gets run exactly once
///   immediately after `anchor_setup`
/// * `update_preview`: This is run each time a [`Hover`] signal arrives. This
///   is where you should put the logic to update the preview that's being displayed
///   for users.
/// * `update_current`: This is run each time a [`Select`] signal containing `Some`
///   value is sent. This is where you should put the logic to make a persistent
///   (rather than just a preview) modification to the world.
/// * `handle_key_code`: This is where you should put the logic for how your
///   workflow responds to various key codes. For example, should the workflow
///   exit?
/// * `cleanup_state`: This is where you should run anything that's needed to
///   clean up the state of the world after your workflow is finished running.
///   This will be run no matter whether your workflow terminates with a success,
///   terminates with a failure, or cancels prematurely.
///
/// ### The remaining parameters can all be provided by [`AnchorSelectionHelpers`] in most cases:
///
/// * `anchor_cursor_transform`: This service should update the 3D cursor transform.
///   A suitable service for this is available from [`AnchorSelectionHelpers`].
/// * `anchor_select_stream`: This service should produce the [`Hover`] and [`Select`]
///   streams that hook into `update_preview` and `update_current` respectively.
///   A suitable service for this is provided by [`AnchorSelectionHelpers`].
/// * `keyobard_just_pressed`: This service should produce [`KeyCode`] streams
///   when the keyboard gets pressed. A suitable service for this is provided by
///   [`AnchorSelectionHelpers`].
/// * `cleanup_anchor_selection`: This service will run during the cleanup phase
///   and should cleanup any anchor-related modifications to the world. A suitable
///   service for this is provided by [`AnchorSelectionHelpers`].
pub fn build_anchor_selection_workflow<State: 'static + Send + Sync>(
    anchor_setup: Service<BufferKey<State>, SelectionNodeResult>,
    state_setup: Service<BufferKey<State>, SelectionNodeResult>,
    update_preview: Service<(Hover, BufferKey<State>), SelectionNodeResult>,
    update_current: Service<(SelectionCandidate, BufferKey<State>), SelectionNodeResult>,
    handle_key_code: Service<((KeyCode, ButtonInputType), BufferKey<State>), SelectionNodeResult>,
    cleanup_state: Service<BufferKey<State>, SelectionNodeResult>,
    anchor_cursor_transform: Service<(), ()>,
    anchor_select_stream: Service<(), (), SelectionStreams>,
    keyboard_pressed: Service<(), (), StreamOf<(KeyCode, ButtonInputType)>>,
    cleanup_anchor_selection: Service<(), ()>,
) -> impl FnOnce(Scope<Option<Entity>, ()>, &mut Builder) {
    move |scope, builder| {
        let buffer = builder.create_buffer::<State>(BufferSettings::keep_last(1));

        let setup_node = builder.create_buffer_access(buffer);
        scope
            .input
            .chain(builder)
            .then(extract_selector_input.into_blocking_callback())
            // If the setup failed, then terminate right away.
            .branch_for_err(|chain: Chain<_>| chain.connect(scope.terminate))
            .fork_option(
                |some: Chain<_>| some.then_push(buffer).connect(setup_node.input),
                |none: Chain<_>| none.connect(setup_node.input),
            );

        let begin_input_services = setup_node
            .output
            .chain(builder)
            .map_block(|(_, key)| key)
            .then(anchor_setup)
            .branch_for_err(|err| err.map_block(print_if_err).connect(scope.terminate))
            .with_access(buffer)
            .map_block(|(_, key)| key)
            .then(state_setup)
            .branch_for_err(|err| err.map_block(print_if_err).connect(scope.terminate))
            .output()
            .fork_clone(builder);

        begin_input_services
            .clone_chain(builder)
            .then(anchor_cursor_transform)
            .unused();

        let select = begin_input_services
            .clone_chain(builder)
            .then_node(anchor_select_stream);
        select
            .streams
            .hover
            .chain(builder)
            .with_access(buffer)
            .then(update_preview)
            .dispose_on_ok()
            .map_block(print_if_err)
            .connect(scope.terminate);

        select
            .streams
            .select
            .chain(builder)
            .map_block(|s| s.0)
            .dispose_on_none()
            .with_access(buffer)
            .then(update_current)
            .dispose_on_ok()
            .map_block(print_if_err)
            .connect(scope.terminate);

        let keyboard = begin_input_services
            .clone_chain(builder)
            .then_node(keyboard_pressed);
        keyboard
            .streams
            .chain(builder)
            .with_access(buffer)
            .then(handle_key_code)
            .dispose_on_ok()
            .map_block(print_if_err)
            .connect(scope.terminate);

        builder.on_cleanup(buffer, move |scope, builder| {
            let state_node = builder.create_node(cleanup_state);
            let anchor_node = builder.create_node(cleanup_anchor_selection);

            builder.connect(scope.input, state_node.input);
            state_node.output.chain(builder).fork_result(
                |ok| ok.connect(anchor_node.input),
                |err| err.map_block(print_if_err).connect(anchor_node.input),
            );

            builder.connect(anchor_node.output, scope.terminate);
        });
    }
}

pub fn print_if_err(err: Option<anyhow::Error>) {
    if let Some(err) = err {
        error!("{err}");
    }
}

pub fn anchor_selection_setup<State: Borrow<AnchorScope>>(
    In(key): In<BufferKey<State>>,
    access: BufferAccess<State>,
    anchors: Query<Entity, With<Anchor>>,
    drawings: Query<(), With<DrawingMarker>>,
    child_of: Query<&'static ChildOf>,
    mut visibility: Query<&'static mut Visibility>,
    mut hidden_anchors: ResMut<HiddenSelectAnchorEntities>,
    mut current_anchor_scope: ResMut<AnchorScope>,
    mut cursor: ResMut<Cursor>,
    mut highlight: ResMut<HighlightAnchors>,
    mut gizmo_blockers: ResMut<GizmoBlockers>,
) -> SelectionNodeResult
where
    State: 'static + Send + Sync,
{
    let access = access.get(&key).or_broken_buffer()?;
    let state = access.newest().or_broken_state()?;
    let scope: &AnchorScope = (&*state).borrow();
    match scope {
        AnchorScope::General | AnchorScope::Site => {
            // If we are working with normal level or site requests, hide all drawing anchors
            for e in anchors.iter().filter(|e| {
                child_of
                    .get(*e)
                    .is_ok_and(|c| drawings.get(c.parent()).is_ok())
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
    gizmo_blockers.selecting = true;

    *current_anchor_scope = *scope;

    cursor.add_mode(SELECT_ANCHOR_MODE_LABEL, &mut visibility);
    set_visibility(cursor.dagger, &mut visibility, true);
    set_visibility(cursor.halo, &mut visibility, true);

    Ok(())
}

pub fn cleanup_anchor_selection(
    In(_): In<()>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut hidden_anchors: ResMut<HiddenSelectAnchorEntities>,
    mut anchor_scope: ResMut<AnchorScope>,
    mut highlight: ResMut<HighlightAnchors>,
    mut gizmo_blockers: ResMut<GizmoBlockers>,
) {
    cursor.remove_mode(SELECT_ANCHOR_MODE_LABEL, &mut visibility);
    set_visibility(cursor.site_anchor_placement, &mut visibility, false);
    set_visibility(cursor.level_anchor_placement, &mut visibility, false);
    for e in hidden_anchors.drawing_anchors.drain() {
        set_visibility(e, &mut visibility, true);
    }

    highlight.0 = false;
    gizmo_blockers.selecting = false;

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

    let Ok(mut e_mut) = world.get_entity_mut(e) else {
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

    e_mut.despawn();

    Ok(Some(input.0))
}

#[derive(SystemParam)]
pub struct AnchorFilter<'w, 's> {
    inspect: InspectorFilter<'w, 's>,
    anchors: Query<'w, 's, (), With<Anchor>>,
    cursor: Res<'w, Cursor>,
    anchor_scope: Res<'w, AnchorScope>,
    workspace: Res<'w, CurrentWorkspace>,
    open_sites: Query<'w, 's, Entity, With<NameOfSite>>,
    transforms: Query<'w, 's, &'static GlobalTransform>,
    commands: Commands<'w, 's>,
    current_drawing: Res<'w, CurrentEditDrawing>,
    drawings: Query<'w, 's, &'static PixelsPerMeter, With<DrawingMarker>>,
    child_of: Query<'w, 's, &'static ChildOf>,
    levels: Query<'w, 's, (), With<LevelElevation>>,
    lifts: Query<
        'w,
        's,
        (
            Entity,
            &'static LiftCabin<Entity>,
            &'static ChildCabinAnchorGroup,
        ),
    >,
    current_level: Res<'w, CurrentLevel>,
}

impl<'w, 's> SelectionFilter for AnchorFilter<'w, 's> {
    fn filter_pick(&mut self, select: Entity) -> Option<Entity> {
        self.inspect
            .filter_pick(select)
            .and_then(|e| self.filter_target(e))
    }

    fn filter_select(&mut self, target: Entity) -> Option<Entity> {
        self.filter_target(target)
    }

    fn on_click(&mut self, hovered: Hover) -> Option<Select> {
        if let Some(candidate) = hovered.0.and_then(|e| self.filter_target(e)) {
            return Some(Select::new(Some(candidate)));
        }

        // There was no anchor currently hovered which means we need to create
        // a new provisional anchor.
        let tf = if let Ok(placement_tf) = self.transforms.get(self.cursor.level_anchor_placement) {
            placement_tf
        } else if let Ok(cursor_tf) = self.transforms.get(self.cursor.frame) {
            cursor_tf
        } else {
            error!("Cannot find cursor transform");
            return None;
        };

        let new_anchor = match self.anchor_scope.as_ref() {
            AnchorScope::Site => {
                let Some(site) = self.workspace.to_site(&self.open_sites) else {
                    error!("Cannot find current site");
                    return None;
                };
                let new_anchor = self
                    .commands
                    .spawn(AnchorBundle::at_transform(tf))
                    .insert(ChildOf(site))
                    .id();
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
                    .insert(ChildOf(drawing))
                    .id()
            }
            AnchorScope::General => {
                let Some(level) = self.current_level.0 else {
                    error!("No current level selected to place the anchor");
                    return None;
                };
                // Check if the anchor is inside of a lift
                let mut lift_anchor = None;
                for (lift, cabin, anchor_group) in &self.lifts {
                    if let Ok(lift_tf) = self.transforms.get(lift) {
                        let affine = lift_tf.compute_transform().compute_affine();
                        let p = affine.inverse().transform_point3a(tf.translation_vec3a());
                        if cabin.contains_point(p) {
                            // The anchor group has a different reference frame
                            // than the lift frame, so transform the anchor into the
                            // anchor group frame.
                            if let Ok(group_tf) = self.transforms.get(**anchor_group) {
                                let affine = group_tf.compute_transform().compute_affine();
                                let p = affine.inverse().transform_point3a(tf.translation_vec3a());
                                lift_anchor = Some(
                                    self.commands
                                        .spawn((
                                            AnchorBundle::new([p[0], p[1]].into()),
                                            ChildOf(**anchor_group),
                                        ))
                                        .id(),
                                );
                                break;
                            }
                        }
                    }
                }

                if let Some(anchor) = lift_anchor {
                    anchor
                } else {
                    self.commands
                        .spawn((AnchorBundle::at_transform(tf), ChildOf(level)))
                        .id()
                }
            }
        };

        Some(Select::provisional(new_anchor))
    }
}

impl<'w, 's> AnchorFilter<'w, 's> {
    fn filter_anchor(&mut self, target: Entity) -> Option<Entity> {
        if self.anchors.contains(target) {
            Some(target)
        } else {
            None
        }
    }

    fn filter_scope(&mut self, target: Entity) -> Option<Entity> {
        let parent = match self.child_of.get(target) {
            Ok(child_of) => child_of.parent(),
            Err(err) => {
                error!("Unable to detect parent for target anchor {target:?}: {err}");
                return None;
            }
        };

        match &*self.anchor_scope {
            AnchorScope::General => {
                let is_site = || self.open_sites.contains(parent);
                let is_level = || self.levels.contains(parent);
                let is_lift =
                    || AncestorIter::new(&self.child_of, target).any(|e| self.lifts.contains(e));
                if is_site() || is_level() || is_lift() {
                    Some(target)
                } else {
                    None
                }
            }
            AnchorScope::Site => {
                if self.open_sites.contains(parent) {
                    Some(target)
                } else {
                    None
                }
            }
            AnchorScope::Drawing => {
                if self.drawings.contains(parent) {
                    Some(target)
                } else {
                    None
                }
            }
        }
    }

    fn filter_target(&mut self, target: Entity) -> Option<Entity> {
        self.filter_anchor(target)
            .and_then(|target| self.filter_scope(target))
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

pub fn exit_on_esc<T>(
    In(((button, input_type), _)): In<((KeyCode, ButtonInputType), BufferKey<T>)>,
) -> SelectionNodeResult {
    if matches!(button, KeyCode::Escape) && matches!(input_type, ButtonInputType::JustPressed) {
        // The escape key was pressed so we should exit this mode
        return Err(None);
    }

    Ok(())
}

/// Update the virtual cursor transform while in select anchor mode
pub fn select_anchor_cursor_transform(
    In(ContinuousService { key }): ContinuousServiceInput<(), ()>,
    orders: ContinuousQuery<(), ()>,
    cursor: Res<Cursor>,
    mut transforms: Query<&mut Transform>,
    intersect_ground_params: IntersectGroundPlaneParams,
) {
    let Some(orders) = orders.view(&key) else {
        return;
    };

    if orders.is_empty() {
        return;
    }

    let intersection = match intersect_ground_params.ground_plane_intersection() {
        Some(intersection) => intersection,
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

    *transform = Transform::from_translation(intersection.translation);
}
