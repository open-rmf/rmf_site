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
    interaction::*,
    site::{
        drawing_editor::CurrentEditDrawing, model::ModelSpawningExt, Anchor, AnchorBundle,
        Category, CollisionMeshMarker, Dependents, DrawingMarker, Original, PathBehavior, Pending,
        TextureNeedsAssignment, VisualMeshMarker,
    },
    AppState, CurrentWorkspace,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use rmf_site_format::{
    Door, Edge, Fiducial, Floor, FrameMarker, Lane, LiftProperties, Location, Measurement, Model,
    NameInWorkcell, NameOfSite, Path, PixelsPerMeter, Point, Pose, Side, Wall, WorkcellModel,
};
use std::collections::HashSet;
use std::sync::Arc;

const SELECT_ANCHOR_MODE_LABEL: &'static str = "select_anchor";

/// Describe how the interaction mode should change while
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectAnchorContinuity {
    /// Only select one anchor to replace one that the element was already
    /// referencing. As soon as one anchor is selected and given to its target,
    /// the interaction mode will revert to Inspect. This variant is not
    /// compatible if target is None, and will be promoted to OneElement
    /// (TODO(MXG): or should we panic instead?).
    ///
    /// Backout from this mode will return to Inspect.
    ///
    /// We allow the original_anchor to initially be None so that users do not
    /// need to know its initial value. The handle_select_anchor_mode will fill
    /// this in when the time comes.
    ReplaceAnchor { original_anchor: Option<Entity> },
    /// Select enough anchors for one element to be completed. For edge-like
    /// elements this means both sides must be filled in. For Location, one
    /// selection is enough. For Floor this will be equivalent to Continuous
    /// except that backing out will delete all progress.
    ///
    /// Backout from this mode when target is None will return to Inspect.
    /// When target is something, Backout will set it to None, allowing the
    /// user to restart creating the element. Any progress creating the previous
    /// element will be lost, although anchors that were created in the process
    /// will remain.
    InsertElement,
    /// Keep selecting anchors indefinitely. When an element has been finished,
    /// a new element will be created starting from the last anchor that was
    /// selected.
    ///
    /// Backout behavior finishes the current element that's being constructed.
    /// If the element did not qualify to finish then it is deleted. The user
    /// can begin drawing a new element. A double-backout is needed to exit the
    /// mode.
    Continuous { previous: Option<Entity> },
}

impl SelectAnchorContinuity {
    fn needs_original(&self) -> bool {
        match self {
            Self::ReplaceAnchor { original_anchor } => {
                return original_anchor.is_none();
            }
            _ => {
                return false;
            }
        }
    }

    fn replacing(&self) -> Option<Entity> {
        match self {
            Self::ReplaceAnchor { original_anchor } => *original_anchor,
            _ => None,
        }
    }

    fn previous(&self) -> Option<Entity> {
        match self {
            Self::Continuous { previous } => *previous,
            _ => None,
        }
    }
}

struct Transition {
    target: TargetTransition,
    placement: PlacementTransition,
}

impl From<(TargetTransition, PlacementTransition)> for Transition {
    fn from(input: (TargetTransition, PlacementTransition)) -> Self {
        Self {
            target: input.0,
            placement: input.1,
        }
    }
}

type CreateEdgeFn =
    Arc<dyn Fn(&mut SelectAnchorPlacementParams, Edge<Entity>) -> Entity + Send + Sync>;
type CreatePointFn =
    Arc<dyn Fn(&mut SelectAnchorPlacementParams, Point<Entity>) -> Entity + Send + Sync>;
type CreatePathFn =
    Arc<dyn Fn(&mut SelectAnchorPlacementParams, Path<Entity>) -> Entity + Send + Sync>;
type FinalizeFn = Arc<dyn Fn(&mut SelectAnchorPlacementParams, Entity) + Send + Sync>;

struct TargetTransition {
    created: Option<Entity>,
    is_finished: bool,
    discontinue: bool,
}

impl TargetTransition {
    fn none() -> Self {
        Self {
            created: None,
            is_finished: false,
            discontinue: false,
        }
    }

    fn create(e: Entity) -> Self {
        Self {
            created: Some(e),
            is_finished: false,
            discontinue: false,
        }
    }

    fn finished() -> Self {
        Self {
            created: None,
            is_finished: true,
            discontinue: false,
        }
    }

    fn discontinued() -> Self {
        Self {
            created: None,
            is_finished: false,
            discontinue: true,
        }
    }

    fn finish(mut self) -> Self {
        self.is_finished = true;
        self
    }

    fn current(&self, e: Option<Entity>) -> Option<Entity> {
        match e {
            Some(e) => {
                if self.created.is_some() {
                    error!(
                        "Created a superfluous target while in \
                        SelectAnchor mode"
                    );
                }
                Some(e)
            }
            None => match self.created {
                Some(e) => Some(e),
                None => {
                    error!(
                        "Failed to create an entity while in \
                            SelectAnchor mode"
                    );
                    None
                }
            },
        }
    }

    fn next(&self, e: Option<Entity>) -> Option<Entity> {
        if self.is_finished {
            return None;
        }

        // The logic for next is the same as preview if the object is not
        // finished yet
        return self.current(e);
    }
}

struct PlacementTransition {
    preview: Option<PlacementArc>,
    next: PlacementArc,
}

trait Placement {
    /// Get what the next placement should be if an anchor is selected for the
    /// current placement. If None is returned, that means the element has been
    /// filled.
    fn next<'w, 's>(
        &self,
        anchor_selection: AnchorSelection,
        continuity: SelectAnchorContinuity,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()>;

    fn current<'w, 's>(
        &self,
        target: Entity,
        params: &SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity>;

    /// Check what anchor originally has this placement
    fn save_original<'w, 's>(
        &self,
        target: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity>;

    fn finalize<'w, 's>(&self, target: Entity, params: &mut SelectAnchorPlacementParams<'w, 's>);

    fn backout<'w, 's>(
        &self,
        continuity: SelectAnchorContinuity,
        target: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()>;
}

/// Because of bevy's async nature, we need to use different methods for
/// modifying anchor dependents based on whether the anchor is brand new or
/// whether the anchor already existed before the current system was run. To
/// simplify that we encapsulate the logic inside of this AnchorSelection enum.
enum AnchorSelection {
    Existing(Entity),
    New {
        entity: Entity,
        dependents: Dependents,
    },
}

impl AnchorSelection {
    fn new(entity: Entity) -> Self {
        Self::New {
            entity,
            dependents: Default::default(),
        }
    }

    fn existing(entity: Entity) -> Self {
        Self::Existing(entity)
    }

    fn entity(&self) -> Entity {
        match self {
            Self::Existing(entity) | Self::New { entity, .. } => *entity,
        }
    }

    fn add_dependent<'w, 's>(
        &mut self,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
        dependent: Entity,
    ) -> Result<(), ()> {
        match self {
            Self::Existing(e) => {
                let mut deps = match params.dependents.get_mut(*e).map_err(|_| ()) {
                    Ok(dep) => dep,
                    Err(_) => {
                        // The entity was not a proper anchor
                        error!("Invalid anchor selected {:?}", e);
                        return Err(());
                    }
                };
                deps.insert(dependent);
                Ok(())
            }
            Self::New { entity, dependents } => {
                dependents.insert(dependent);
                params.commands.entity(*entity).insert(dependents.clone());
                Ok(())
            }
        }
    }

    fn remove_dependent<'w, 's>(
        &mut self,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
        dependent: Entity,
    ) -> Result<(), ()> {
        match self {
            Self::Existing(e) => {
                let mut deps = match params.dependents.get_mut(*e).map_err(|_| ()) {
                    Ok(dep) => dep,
                    Err(_) => {
                        error!("Invalid anchor selected {:?}", e);
                        return Err(());
                    }
                };
                deps.remove(&dependent);
                Ok(())
            }
            Self::New { entity, dependents } => {
                dependents.remove(&dependent);
                params.commands.entity(*entity).insert(dependents.clone());
                Ok(())
            }
        }
    }
}

#[derive(Clone)]
pub struct EdgePlacement {
    side: Side,
    create: CreateEdgeFn,
    finalize: FinalizeFn,
}

impl EdgePlacement {
    fn to_start(&self) -> PlacementTransition {
        PlacementTransition {
            preview: None,
            next: Arc::new(Self {
                side: Side::Left,
                create: self.create.clone(),
                finalize: self.finalize.clone(),
            }),
        }
    }

    fn to_end(&self) -> PlacementTransition {
        PlacementTransition {
            preview: None,
            next: Arc::new(Self {
                side: Side::Right,
                create: self.create.clone(),
                finalize: self.finalize.clone(),
            }),
        }
    }

    fn ignore(&self) -> PlacementTransition {
        PlacementTransition {
            preview: None,
            next: Arc::new(self.clone()),
        }
    }

    fn new<T: Bundle + From<Edge<Entity>>>(side: Side) -> Arc<Self> {
        Arc::new(Self {
            side,
            create: Arc::new(
                |params: &mut SelectAnchorPlacementParams, edge: Edge<Entity>| {
                    let new_bundle: T = edge.into();
                    params.commands.spawn(new_bundle).insert(Pending).id()
                },
            ),
            finalize: Arc::new(|params: &mut SelectAnchorPlacementParams, entity: Entity| {
                params
                    .commands
                    .entity(entity)
                    .remove::<Original<Edge<Entity>>>();
            }),
        })
    }

    fn with_extra<F>(self: Arc<Self>, f: F) -> Arc<Self>
    where
        F: Fn(&mut SelectAnchorPlacementParams, Entity) + Send + Sync + 'static,
    {
        let mut result = match Arc::try_unwrap(self) {
            Ok(r) => r,
            Err(r) => (*r).clone(),
        };
        let base = result.create;
        result.create = Arc::new(
            move |params: &mut SelectAnchorPlacementParams, edge: Edge<Entity>| {
                let entity = base(params, edge);
                f(params, entity);
                entity
            },
        );
        Arc::new(result)
    }

    fn update_dependencies(
        mut anchor_selection: Option<&mut AnchorSelection>,
        target: Entity,
        old_edge: Edge<Entity>,
        new_edge: Edge<Entity>,
        params: &mut SelectAnchorPlacementParams,
    ) -> Result<(), ()> {
        // Remove the target edge as a dependency from any anchors that are no
        // longer being used by this edge.
        for old_anchor in old_edge.array() {
            if new_edge
                .array()
                .iter()
                .find(|x| **x == old_anchor)
                .is_none()
            {
                // This anchor is no longer being used by the edge.
                match params.remove_dependent(target, old_anchor, &mut anchor_selection) {
                    Ok(_) => {
                        // Do nothing
                    }
                    Err(_) => {
                        error!(
                            "No AnchorDependents component found for \
                            {:?} while in SelectAnchor mode.",
                            old_anchor
                        );
                    }
                }
            }
        }

        for new_anchor in new_edge.array() {
            if old_edge
                .array()
                .iter()
                .find(|x| **x == new_anchor)
                .is_none()
            {
                // This anchor was not being used by the edge previously.
                params.add_dependent(target, new_anchor, &mut anchor_selection)?;
            }
        }

        Ok(())
    }
}

impl Placement for EdgePlacement {
    fn next<'w, 's>(
        &self,
        mut anchor_selection: AnchorSelection,
        continuity: SelectAnchorContinuity,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()> {
        let (target, mut endpoints, original) = match target {
            Some(target) => {
                // We expect that we already have an element and we are
                // modifying it.
                match params.edges.get_mut(target) {
                    Ok((edge, original)) => (target, edge, original),
                    Err(_) => {
                        error!(
                            "Entity {:?} is not the right kind of \
                            element",
                            target,
                        );
                        return Err(());
                    }
                }
            }
            None => {
                // We need to begin creating a new element
                if self.side == Side::Right {
                    if let Some(previous) = continuity.previous() {
                        // We should connect this new element to the previous one.
                        let (previous_edge, _) = params.edges.get(previous).map_err(|_| ())?;
                        let anchors = Edge::new(previous_edge.right(), anchor_selection.entity());
                        let target = (*self.create)(params, anchors);
                        for anchor in anchors.array() {
                            params.add_dependent(
                                target,
                                anchor,
                                &mut Some(&mut anchor_selection),
                            )?;
                        }

                        return Ok(
                            (TargetTransition::create(target).finish(), self.to_end()).into()
                        );
                    }
                }

                let anchors = Edge::new(
                    anchor_selection.entity(),
                    params.cursor.level_anchor_placement,
                );
                let target = (*self.create)(params, anchors);
                for anchor in anchors.array() {
                    params.add_dependent(target, anchor, &mut Some(&mut anchor_selection))?;
                }
                return Ok((TargetTransition::create(target), self.to_end()).into());
            }
        };

        let new_edge = match original {
            Some(original) => {
                if anchor_selection.entity() == original.side(self.side.opposite()) {
                    // The user is asking to swap the anchors
                    original.in_reverse()
                } else {
                    original.with_side_of(self.side, anchor_selection.entity())
                }
            }
            None => {
                match continuity.replacing() {
                    Some(replacing) => {
                        let new_edge = if endpoints.side(self.side.opposite()) == replacing {
                            // The opposite anchor was assigned the anchor that
                            // is being replaced. This implies that a flip
                            // happened at some point in the past. We should
                            // flip the edge back to normal before continuing.
                            *endpoints = endpoints.in_reverse();
                            *endpoints
                        } else {
                            *endpoints
                        };

                        new_edge.with_side_of(self.side, anchor_selection.entity())
                    }
                    None => {
                        match self.side {
                            Side::Left => {
                                // We are reseting the start point of an edge
                                // that is being freshly created, so set both
                                // sides to the same anchor.
                                Edge::new(anchor_selection.entity(), anchor_selection.entity())
                            }
                            Side::Right => {
                                if endpoints.left() == anchor_selection.entity() {
                                    // The user is asking to select the same anchor for
                                    // both sides of the edge. This is not okay, so we
                                    // ignore this request.
                                    return Ok((TargetTransition::none(), self.ignore()).into());
                                }

                                endpoints.with_side_of(Side::Right, anchor_selection.entity())
                            }
                        }
                    }
                }
            }
        };

        let old_edge = *endpoints;
        *endpoints = new_edge;
        Self::update_dependencies(
            Some(&mut anchor_selection),
            target,
            old_edge,
            new_edge,
            params,
        )?;

        return match self.side {
            Side::Left => Ok((TargetTransition::none(), self.to_end()).into()),
            Side::Right => match continuity {
                SelectAnchorContinuity::Continuous { .. } => {
                    Ok((TargetTransition::finished(), self.to_end()).into())
                }
                _ => Ok((TargetTransition::finished(), self.to_start()).into()),
            },
        };
    }

    fn current<'w, 's>(
        &self,
        target: Entity,
        params: &SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> {
        params
            .edges
            .get(target)
            .ok()
            .map(|edge| edge.0.side(self.side))
    }

    fn save_original<'w, 's>(
        &self,
        target: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> {
        if let Ok(original) = params.edges.get(target).map(|x| x.0).cloned() {
            params.commands.entity(target).insert(Original(original));
            return Some(original.side(self.side));
        }

        return None;
    }

    fn finalize<'w, 's>(&self, target: Entity, params: &mut SelectAnchorPlacementParams<'w, 's>) {
        (*self.finalize)(params, target);
    }

    fn backout<'w, 's>(
        &self,
        continuity: SelectAnchorContinuity,
        target: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()> {
        // Restore visibility to anchors that were hidden in this mode
        for e in params.hidden_entities.drawing_anchors.drain() {
            set_visibility(e, &mut params.visibility, true);
        }
        if continuity.replacing().is_some() {
            // Restore the target to its original and then quit
            if let Ok((mut edge, original)) = params.edges.get_mut(target) {
                if let Some(original) = original {
                    let old_edge = *edge;
                    *edge = **original;
                    Self::update_dependencies(None, target, old_edge, *edge, params)?;
                    return Ok((TargetTransition::finished(), self.to_start()).into());
                } else {
                    error!(
                        "Unable to find original for {target:?} \
                        while backing out of edge replacement"
                    );
                    return Err(());
                }
            } else {
                error!(
                    "Unable to find edge for {target:?} while \
                    backing out of edge replacement"
                );
                return Err(());
            }
        } else {
            // Delete the target because it is unfinished, then restart from
            // the beginning.
            let equal_points = if let Ok((edge, _)) = params.edges.get(target) {
                for anchor in edge.array() {
                    if let Ok(mut deps) = params.dependents.get_mut(anchor) {
                        deps.remove(&target);
                    }
                }
                edge.start() == edge.end()
            } else {
                true
            };

            params.commands.entity(target).despawn_recursive();

            if equal_points {
                return Ok((TargetTransition::discontinued(), self.to_start()).into());
            } else {
                return Ok((TargetTransition::none(), self.to_start()).into());
            }
        }
    }
}

#[derive(Clone)]
pub struct PointPlacement {
    create: CreatePointFn,
    finalize: FinalizeFn,
}

impl PointPlacement {
    fn new<T: Bundle + From<Point<Entity>>>() -> Arc<Self> {
        Arc::new(Self {
            create: Arc::new(
                |params: &mut SelectAnchorPlacementParams, point: Point<Entity>| {
                    let new_bundle: T = point.into();
                    params.commands.spawn(new_bundle).insert(Pending).id()
                },
            ),
            finalize: Arc::new(|params: &mut SelectAnchorPlacementParams, entity: Entity| {
                params
                    .commands
                    .entity(entity)
                    .remove::<Original<Point<Entity>>>();
            }),
        })
    }
}

impl PointPlacement {
    fn transition(&self) -> PlacementTransition {
        PlacementTransition {
            preview: None,
            next: Arc::new(self.clone()),
        }
    }
}

impl Placement for PointPlacement {
    fn next<'w, 's>(
        &self,
        mut anchor_selection: AnchorSelection,
        _continuity: SelectAnchorContinuity,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()> {
        match target {
            Some(target) => {
                // Change the anchor that the location is attached to.
                let mut point = match params.points.get_mut(target) {
                    Ok(l) => l,
                    Err(_) => {
                        error!(
                            "Unable to get location {:?} while in \
                            SelectAnchor mode.",
                            target
                        );
                        return Err(());
                    }
                };

                if **point != anchor_selection.entity() {
                    let old_point = **point;
                    **point = anchor_selection.entity();
                    params.remove_dependent(target, old_point, &mut Some(&mut anchor_selection))?;
                    params.add_dependent(
                        target,
                        anchor_selection.entity(),
                        &mut Some(&mut anchor_selection),
                    )?;
                }

                return Ok((TargetTransition::finished(), self.transition()).into());
            }
            None => {
                // The element doesn't exist yet, so we need to spawn one.
                let target = (*self.create)(params, Point(anchor_selection.entity()));
                params.add_dependent(
                    target,
                    anchor_selection.entity(),
                    &mut Some(&mut anchor_selection),
                )?;
                return Ok((TargetTransition::create(target).finish(), self.transition()).into());
            }
        }
    }

    fn current<'w, 's>(
        &self,
        target: Entity,
        params: &SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> {
        params.points.get(target).ok().map(|p| p.0)
    }

    fn save_original<'w, 's>(
        &self,
        target: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> {
        if let Ok(original) = params.points.get(target).cloned() {
            params.commands.entity(target).insert(Original(original));
            return Some(original.0);
        }

        return None;
    }

    fn finalize<'w, 's>(&self, target: Entity, params: &mut SelectAnchorPlacementParams<'w, 's>) {
        (*self.finalize)(params, target);
    }

    fn backout<'w, 's>(
        &self,
        continuity: SelectAnchorContinuity,
        target: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()> {
        if let Ok(mut point) = params.points.get_mut(target) {
            if let Ok(mut deps) = params.dependents.get_mut(**point) {
                deps.remove(&target);
            }

            if let Some(replacing) = continuity.replacing() {
                // Restore the target to the original
                if let Ok(mut deps) = params.dependents.get_mut(replacing) {
                    deps.insert(target);
                }

                point.0 = replacing;
                return Ok((TargetTransition::finished(), self.transition()).into());
            } else {
                // Delete the location entirely because there is no anchor to
                // return it to.
                params.commands.entity(target).despawn_recursive();
                return Ok((TargetTransition::discontinued(), self.transition()).into());
            }
        } else {
            error!(
                "Cannot find point for location {target:?} while \
                trying to back out of SelectAnchor mode"
            );
            return Err(());
        }
    }
}

#[derive(Clone)]
pub struct PathPlacement {
    /// Replace the floor anchor at the specified index, or push the anchor to
    /// the end if None is specified. If the specified index is too high, this
    /// value will be changed to None and all new anchors will be pushed to the
    /// back.
    index: Option<usize>,
    create: CreatePathFn,
    finalize: FinalizeFn,
}

impl PathPlacement {
    fn new<T: Bundle + From<Path<Entity>>>(placement: Option<usize>) -> Arc<Self> {
        Arc::new(Self {
            index: placement,
            create: Arc::new(
                |params: &mut SelectAnchorPlacementParams, path: Path<Entity>| {
                    let new_bundle: T = path.into();
                    params.commands.spawn(new_bundle).insert(Pending).id()
                },
            ),
            finalize: Arc::new(|params: &mut SelectAnchorPlacementParams, entity: Entity| {
                params
                    .commands
                    .entity(entity)
                    .remove::<Original<Path<Entity>>>();
            }),
        })
    }

    fn with_extra<F>(self: Arc<Self>, f: F) -> Arc<Self>
    where
        F: Fn(&mut SelectAnchorPlacementParams, Entity) + Send + Sync + 'static,
    {
        let mut result = match Arc::try_unwrap(self) {
            Ok(r) => r,
            Err(r) => (*r).clone(),
        };
        let base = result.create;
        result.create = Arc::new(
            move |params: &mut SelectAnchorPlacementParams, path: Path<Entity>| {
                let entity = base(params, path);
                f(params, entity);
                entity
            },
        );
        Arc::new(result)
    }

    fn at_index(&self, index: usize) -> Arc<Self> {
        Arc::new(Self {
            index: Some(index),
            create: self.create.clone(),
            finalize: self.finalize.clone(),
        })
    }

    fn restart(&self) -> PlacementTransition {
        PlacementTransition {
            preview: None,
            next: Arc::new(Self {
                index: None,
                create: self.create.clone(),
                finalize: self.finalize.clone(),
            }),
        }
    }

    fn transition_from(&self, index: usize) -> PlacementTransition {
        PlacementTransition {
            preview: Some(self.at_index(index)),
            next: self.at_index(index + 1),
        }
    }

    fn ignore(&self) -> PlacementTransition {
        PlacementTransition {
            preview: None,
            next: Arc::new(self.clone()),
        }
    }
}

impl Placement for PathPlacement {
    fn next<'w, 's>(
        &self,
        mut anchor_selection: AnchorSelection,
        _continuity: SelectAnchorContinuity,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()> {
        let target = match target {
            Some(target) => target,
            None => {
                // We need to create a new element
                let target = (*self.create)(params, Path(vec![anchor_selection.entity()]));
                params.add_dependent(
                    target,
                    anchor_selection.entity(),
                    &mut Some(&mut anchor_selection),
                )?;
                return Ok((TargetTransition::create(target), self.transition_from(0)).into());
            }
        };

        let (mut path, behavior) = match params.paths.get_mut(target) {
            Ok(q) => q,
            Err(_) => {
                error!(
                    "Unable to find path info for {target:?} while \
                    in SelectAnchor mode."
                );
                return Err(());
            }
        };

        let index = self.index.unwrap_or(path.len()).min(path.len());
        if path.len() >= behavior.minimum_points && behavior.implied_complete_loop {
            if Some(anchor_selection.entity()) == path.first().cloned() {
                if index >= path.len() - 1 {
                    // The user has set the first node to the last node,
                    // creating a closed loop. We should consider the floor to
                    // be finished.
                    if index == path.len() - 1 {
                        // Remove the last element because it is redundant with
                        // the first element now.
                        path.pop();
                    }
                    return Ok((TargetTransition::finished(), self.transition_from(index)).into());
                }
            }
        }

        if !behavior.allow_inner_loops {
            for (i, anchor) in path.iter().enumerate() {
                if *anchor == anchor_selection.entity() && i != index {
                    // The user has reselected a midpoint. That violates the
                    // requested behavior, so we ignore it.
                    return Ok((TargetTransition::none(), self.ignore()).into());
                }
            }
        }

        if let Some(place_anchor) = path.get_mut(index) {
            let old_anchor = *place_anchor;
            *place_anchor = anchor_selection.entity();

            if path.iter().find(|x| **x == old_anchor).is_none() {
                params.remove_dependent(target, old_anchor, &mut Some(&mut anchor_selection))?;
            }
        } else {
            // We need to add this anchor to the end of the vector
            path.push(anchor_selection.entity());
        }

        params.add_dependent(
            target,
            anchor_selection.entity(),
            &mut Some(&mut anchor_selection),
        )?;
        return Ok((TargetTransition::none(), self.transition_from(index)).into());
    }

    fn current<'w, 's>(
        &self,
        target: Entity,
        params: &SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> {
        let index = match self.index {
            Some(i) => i,
            None => {
                return None;
            }
        };

        let path = match params.paths.get(target) {
            Ok(p) => p.0,
            Err(_) => {
                error!(
                    "Unable to find path for {:?} while in \
                    SelectAnchor mode",
                    target,
                );
                return None;
            }
        };

        path.get(index).cloned()
    }

    fn save_original<'w, 's>(
        &self,
        target: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> {
        let path = match params.paths.get(target) {
            Ok(p) => p.0.clone(),
            Err(_) => {
                error!(
                    "Unable to find path for {:?} while in \
                    SelectAnchor mode",
                    target,
                );
                return None;
            }
        };

        let placement = self.index.map(|index| path.get(index).cloned()).flatten();
        params.commands.entity(target).insert(Original(path));
        return placement;
    }

    fn finalize<'w, 's>(&self, target: Entity, params: &mut SelectAnchorPlacementParams<'w, 's>) {
        (*self.finalize)(params, target);
    }

    fn backout<'w, 's>(
        &self,
        continuity: SelectAnchorContinuity,
        target: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()> {
        if let Some(replacing) = continuity.replacing() {
            if let Some(index) = self.index {
                let (mut path, _) = params.paths.get_mut(target).map_err(|_| ())?;
                if let Some(anchor) = path.get_mut(index) {
                    let mut deps = params.dependents.get_mut(*anchor).map_err(|_| ())?;
                    deps.remove(&target);
                    deps.insert(replacing);
                    *anchor = replacing;

                    return Ok((TargetTransition::finished(), self.restart()).into());
                }

                error!(
                    "Path of length {} is missing the index {} \
                    that was supposed to be replaced.",
                    path.len(),
                    index
                );
                return Err(());
            }

            error!(
                "Unable to find the placement of a path anchor \
                that is being replaced."
            );
            return Err(());
        }

        let (mut path, behavior) = params.paths.get_mut(target).map_err(|_| ())?;
        let discontinue = path.is_empty() || (path.len() == 1 && self.index == Some(0));

        let insufficient_points = path.len() < behavior.minimum_points
            || (path.len() < behavior.minimum_points + 1
                && self.index < Some(behavior.minimum_points));

        if insufficient_points || discontinue {
            // We're backing out when the path is too small, so we will delete
            // the object.
            for anchor in path.iter() {
                if let Ok(mut deps) = params.dependents.get_mut(*anchor) {
                    deps.remove(&target);
                }
            }

            params.commands.entity(target).despawn_recursive();
            if discontinue {
                return Ok((TargetTransition::discontinued(), self.restart()).into());
            }
            return Ok((TargetTransition::none(), self.restart()).into());
        }

        if let Some(last) = path.last() {
            // The last anchor is virtually guaranteed to be one that's being
            // previewed rather than one that's actually been selected, so we
            // will remove it here. Technically there could be a race condition
            // if the user selects + backs out in the same update cycle, but if
            // they're giving conflicting inputs in such a small window then
            // it's not unreasonable for us to permit that race condition.
            let mut deps = params.dependents.get_mut(*last).unwrap();
            deps.remove(last);
            path.pop();
        }

        return Ok((TargetTransition::finished(), self.restart()).into());
    }
}

#[derive(Resource, Default)]
pub struct HiddenSelectAnchorEntities {
    /// All drawing anchors, hidden when users draw level entities such as walls, lanes, floors to
    /// make sure they don't connect to drawing anchors
    pub drawing_anchors: HashSet<Entity>,
}

#[derive(SystemParam)]
pub struct SelectAnchorPlacementParams<'w, 's> {
    edges: Query<
        'w,
        's,
        (
            &'static mut Edge<Entity>,
            Option<&'static Original<Edge<Entity>>>,
        ),
    >,
    points: Query<'w, 's, &'static mut Point<Entity>>,
    anchors: Query<'w, 's, (Entity, &'static mut Anchor)>,
    parents: Query<'w, 's, &'static mut Parent>,
    paths: Query<'w, 's, (&'static mut Path<Entity>, &'static PathBehavior)>,
    dependents: Query<'w, 's, &'static mut Dependents>,
    commands: Commands<'w, 's>,
    cursor: ResMut<'w, Cursor>,
    visibility: Query<'w, 's, &'static mut Visibility>,
    drawings: Query<'w, 's, (Entity, &'static PixelsPerMeter), With<DrawingMarker>>,
    hidden_entities: ResMut<'w, HiddenSelectAnchorEntities>,
}

impl<'w, 's> SelectAnchorPlacementParams<'w, 's> {
    fn add_dependent(
        &mut self,
        dependent: Entity,
        to_anchor: Entity,
        anchor_selection: &mut Option<&mut AnchorSelection>,
    ) -> Result<(), ()> {
        if let Some(anchor_selection) = anchor_selection {
            if to_anchor == anchor_selection.entity() {
                return anchor_selection.add_dependent(self, dependent);
            }
        }

        let mut deps = match self.dependents.get_mut(to_anchor).map_err(|_| ()) {
            Ok(dep) => dep,
            Err(_) => {
                error!(
                    "Trying to insert invalid anchor \
                    {to_anchor:?} into entity {dependent:?}"
                );
                return Err(());
            }
        };
        deps.insert(dependent);
        Ok(())
    }

    fn remove_dependent(
        &mut self,
        dependent: Entity,
        from_anchor: Entity,
        anchor_selection: &mut Option<&mut AnchorSelection>,
    ) -> Result<(), ()> {
        if let Some(anchor_selection) = anchor_selection {
            if from_anchor == anchor_selection.entity() {
                return anchor_selection.remove_dependent(self, dependent);
            }
        }

        let mut deps = match self.dependents.get_mut(from_anchor).map_err(|_| ()) {
            Ok(dep) => dep,
            Err(_) => {
                error!(
                    "Removing invalid anchor {from_anchor:?} \
                    from entity {dependent:?}"
                );
                return Err(());
            }
        };
        deps.remove(&dependent);
        Ok(())
    }

    /// Use this when exiting SelectAnchor mode
    fn cleanup(&mut self) {
        self.cursor
            .remove_mode(SELECT_ANCHOR_MODE_LABEL, &mut self.visibility);
        set_visibility(
            self.cursor.site_anchor_placement,
            &mut self.visibility,
            false,
        );
        set_visibility(
            self.cursor.level_anchor_placement,
            &mut self.visibility,
            false,
        );
        set_visibility(self.cursor.frame_placement, &mut self.visibility, false);
        self.cursor.set_model_preview(&mut self.commands, None);
        for e in self.hidden_entities.drawing_anchors.drain() {
            set_visibility(e, &mut self.visibility, true);
        }
    }
}

pub struct SelectAnchorEdgeBuilder {
    for_element: Option<Entity>,
    placement: Side,
    continuity: SelectAnchorContinuity,
}

impl SelectAnchorEdgeBuilder {
    pub fn for_lane(self) -> SelectAnchor {
        SelectAnchor {
            target: self.for_element,
            placement: EdgePlacement::new::<Lane<Entity>>(self.placement),
            continuity: self.continuity,
            scope: Scope::General,
        }
    }

    pub fn for_measurement(self) -> SelectAnchor {
        SelectAnchor {
            target: self.for_element,
            placement: EdgePlacement::new::<Measurement<Entity>>(self.placement),
            continuity: self.continuity,
            scope: Scope::Drawing,
        }
    }

    pub fn for_wall(self) -> SelectAnchor {
        SelectAnchor {
            target: self.for_element,
            placement: EdgePlacement::new::<Wall<Entity>>(self.placement).with_extra(
                |params, entity| {
                    params
                        .commands
                        .entity(entity)
                        .insert(TextureNeedsAssignment);
                },
            ),
            continuity: self.continuity,
            scope: Scope::General,
        }
    }

    pub fn for_door(self) -> SelectAnchor {
        SelectAnchor {
            target: self.for_element,
            placement: EdgePlacement::new::<Door<Entity>>(self.placement),
            continuity: self.continuity,
            scope: Scope::General,
        }
    }

    pub fn for_lift(self) -> SelectAnchor {
        SelectAnchor {
            target: self.for_element,
            placement: EdgePlacement::new::<LiftProperties<Entity>>(self.placement),
            continuity: self.continuity,
            scope: Scope::Site,
        }
    }

    pub fn for_category(self, category: Category) -> Option<SelectAnchor> {
        match category {
            Category::Lane => Some(self.for_lane()),
            Category::Measurement => Some(self.for_measurement()),
            Category::Wall => Some(self.for_wall()),
            Category::Door => Some(self.for_door()),
            Category::Lift => Some(self.for_lift()),
            _ => None,
        }
    }
}

pub struct SelectAnchorPointBuilder {
    for_element: Option<Entity>,
    continuity: SelectAnchorContinuity,
}

impl SelectAnchorPointBuilder {
    pub fn for_location(self) -> SelectAnchor {
        SelectAnchor {
            target: self.for_element,
            placement: PointPlacement::new::<Location<Entity>>(),
            continuity: self.continuity,
            scope: Scope::General,
        }
    }

    pub fn for_site_fiducial(self) -> SelectAnchor {
        SelectAnchor {
            target: self.for_element,
            placement: PointPlacement::new::<Fiducial<Entity>>(),
            continuity: self.continuity,
            scope: Scope::Site,
        }
    }

    pub fn for_drawing_fiducial(self) -> SelectAnchor {
        SelectAnchor {
            target: self.for_element,
            placement: PointPlacement::new::<Fiducial<Entity>>(),
            continuity: self.continuity,
            scope: Scope::Drawing,
        }
    }

    pub fn for_model(self, model: Model) -> SelectAnchor3D {
        SelectAnchor3D {
            bundle: PlaceableObject::Model(model),
            parent: None,
            target: self.for_element,
            continuity: self.continuity,
        }
    }

    pub fn for_visual(self, model: WorkcellModel) -> SelectAnchor3D {
        SelectAnchor3D {
            bundle: PlaceableObject::VisualMesh(model),
            parent: None,
            target: self.for_element,
            continuity: self.continuity,
        }
    }

    pub fn for_collision(self, model: WorkcellModel) -> SelectAnchor3D {
        SelectAnchor3D {
            bundle: PlaceableObject::CollisionMesh(model),
            parent: None,
            target: self.for_element,
            continuity: self.continuity,
        }
    }

    pub fn for_anchor(self, parent: Option<Entity>) -> SelectAnchor3D {
        SelectAnchor3D {
            bundle: PlaceableObject::Anchor,
            parent: parent,
            target: self.for_element,
            continuity: self.continuity,
        }
    }
}

pub struct SelectAnchorPathBuilder {
    for_element: Option<Entity>,
    placement: Option<usize>,
    continuity: SelectAnchorContinuity,
}

impl SelectAnchorPathBuilder {
    pub fn for_floor(self) -> SelectAnchor {
        SelectAnchor {
            target: self.for_element,
            placement: PathPlacement::new::<Floor<Entity>>(self.placement).with_extra(
                |params, entity| {
                    params
                        .commands
                        .entity(entity)
                        .insert(TextureNeedsAssignment);
                },
            ),
            continuity: self.continuity,
            scope: Scope::General,
        }
    }
}

type PlacementArc = Arc<dyn Placement + Send + Sync>;

#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Scope {
    Drawing,
    General,
    Site,
}

impl Scope {
    pub fn is_site(&self) -> bool {
        match self {
            Scope::Site => true,
            _ => false,
        }
    }
}

/// This enum requests that the next selection should be an anchor, and that
/// selection should be provided to one of the enumerated entities. When the
/// inner object is None, that means the selection action should create a new
/// instance of one.
#[derive(Clone)]
pub struct SelectAnchor {
    target: Option<Entity>,
    placement: PlacementArc,
    continuity: SelectAnchorContinuity,
    scope: Scope,
}

impl SelectAnchor {
    pub fn site_scope(&self) -> bool {
        self.scope.is_site()
    }

    pub fn replace_side(edge: Entity, side: Side) -> SelectAnchorEdgeBuilder {
        SelectAnchorEdgeBuilder {
            for_element: Some(edge),
            placement: side,
            continuity: SelectAnchorContinuity::ReplaceAnchor {
                original_anchor: None,
            },
        }
    }

    /// Create a single new element of some edge-like type, e.g. Lane, Wall.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mode = SelectAnchor::create_one_new_edge().for_lane();
    /// ```
    pub fn create_one_new_edge() -> SelectAnchorEdgeBuilder {
        SelectAnchorEdgeBuilder {
            for_element: None,
            placement: Side::Left,
            continuity: SelectAnchorContinuity::InsertElement,
        }
    }

    /// Creates a new path of elements for some edge-like type, e.g. Lane, Wall.
    /// New elements will be continuously produced until the user backs out.
    ///
    /// # Examples
    ///
    /// ```ignore
    /// let mode = SelectAnchor::create_new_path().for_wall();
    /// ```
    pub fn create_new_edge_sequence() -> SelectAnchorEdgeBuilder {
        SelectAnchorEdgeBuilder {
            for_element: None,
            placement: Side::Left,
            continuity: SelectAnchorContinuity::Continuous { previous: None },
        }
    }

    /// Create one new location. After an anchor is selected the new location
    /// will be created and the mode will return to Inspect.
    pub fn create_new_point() -> SelectAnchorPointBuilder {
        SelectAnchorPointBuilder {
            for_element: None,
            continuity: SelectAnchorContinuity::InsertElement,
        }
    }

    /// Move an existing location to a new anchor.
    // TODO(MXG): Make this accessible from the UI
    pub fn replace_point(location: Entity, _original_anchor: Entity) -> SelectAnchorPointBuilder {
        SelectAnchorPointBuilder {
            for_element: Some(location),
            continuity: SelectAnchorContinuity::ReplaceAnchor {
                original_anchor: None,
            },
        }
    }

    /// Create a new floor. The user will be able to select anchors continuously
    /// until they Backout. If the user selects an anchor that is already part
    /// of the floor the selection will be ignored, unless it is the first
    /// anchor of the floor, in which case a Backout will occur.
    pub fn create_new_path() -> SelectAnchorPathBuilder {
        SelectAnchorPathBuilder {
            for_element: None,
            placement: None,
            continuity: SelectAnchorContinuity::Continuous { previous: None },
        }
    }

    /// Replace which anchor one of the points on the floor is using.
    pub fn replace_path_point(path: Entity, index: usize) -> SelectAnchorPathBuilder {
        SelectAnchorPathBuilder {
            for_element: Some(path),
            placement: Some(index),
            continuity: SelectAnchorContinuity::ReplaceAnchor {
                original_anchor: None,
            },
        }
    }

    pub fn extend_path(path: Entity) -> SelectAnchorPathBuilder {
        SelectAnchorPathBuilder {
            for_element: Some(path),
            placement: None,
            continuity: SelectAnchorContinuity::InsertElement,
        }
    }

    /// Whether a new object is being created
    pub fn begin_creating(&self) -> bool {
        match self.continuity {
            SelectAnchorContinuity::ReplaceAnchor { .. } => false,
            SelectAnchorContinuity::InsertElement | SelectAnchorContinuity::Continuous { .. } => {
                self.target.is_none()
            }
        }
    }

    /// Get what the next mode should be if an anchor is selected during the
    /// current mode. If None is returned, that means we are done selecting
    /// anchors and should return to Inspect mode.
    fn next<'w, 's>(
        &self,
        anchor_selection: AnchorSelection,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Self> {
        let transition =
            match self
                .placement
                .next(anchor_selection, self.continuity, self.target, params)
            {
                Ok(t) => t,
                Err(_) => {
                    return None;
                }
            };

        if transition.target.is_finished || self.continuity.replacing().is_some() {
            if let Some(finished_target) = transition.target.current(self.target) {
                // Remove the Pending marker from the target because it has
                // been finished.
                params.commands.entity(finished_target).remove::<Pending>();
                self.placement.finalize(finished_target, params);
            } else {
                error!(
                    "An element was supposed to be finished by \
                    SelectAnchor, but we could not find it"
                );
            }
        }

        let next_target = transition.target.next(self.target);
        let next_placement = transition.placement.next;

        match self.continuity {
            SelectAnchorContinuity::ReplaceAnchor { .. } => {
                // No matter what gets returned for next_target or next_placement
                // we exit the ReplaceAnchor mode as soon as a selection is made.
                return None;
            }
            SelectAnchorContinuity::InsertElement => {
                if transition.target.is_finished {
                    // For InsertElement mode we exit the SelectAnchor mode as
                    // soon as a target is finished.
                    return None;
                } else {
                    return Some(Self {
                        target: next_target,
                        placement: next_placement,
                        continuity: self.continuity,
                        scope: self.scope,
                    });
                }
            }
            SelectAnchorContinuity::Continuous { .. } => {
                return Some(Self {
                    target: next_target,
                    placement: next_placement,
                    continuity: if transition.target.is_finished {
                        // If the target finished then the current target becomes
                        // the previous target.
                        let previous = transition.target.current(self.target);
                        SelectAnchorContinuity::Continuous { previous }
                    } else {
                        // If the target is not finished then we just carry along
                        // the previous continuity.
                        self.continuity.clone()
                    },
                    scope: self.scope,
                });
            }
        }
    }

    fn preview<'w, 's>(
        &self,
        anchor_selection: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> PreviewResult {
        let transition = match self.placement.next(
            AnchorSelection::existing(anchor_selection),
            self.continuity,
            self.target,
            params,
        ) {
            Ok(t) => t,
            Err(_) => {
                return PreviewResult::Invalid;
            }
        };

        let target = match transition.target.current(self.target) {
            Some(target) => target,
            None => {
                // This shouldn't happen. If a target wasn't already assigned
                // then a new one should have been created during the preview.
                // We'll just indicate that we should exit the current mode by
                // returning None.
                return PreviewResult::Invalid;
            }
        };

        if let Some(new_placement) = transition.placement.preview {
            return PreviewResult::Updated(Self {
                target: Some(target),
                placement: new_placement.clone(),
                continuity: self.continuity,
                scope: self.scope,
            });
        }

        if Some(target) == self.target {
            // Neither the placement nor the target has changed due to this
            // preview, so just return the Unchanged variant.
            return PreviewResult::Unchanged;
        }

        return PreviewResult::Updated(Self {
            target: Some(target),
            placement: self.placement.clone(),
            continuity: self.continuity,
            scope: self.scope,
        });
    }

    pub fn backout<'w, 's>(
        &self,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> InteractionMode {
        if let Some(target) = self.target {
            let transition = match self.placement.backout(self.continuity, target, params) {
                Ok(t) => t,
                Err(_) => {
                    params.cleanup();
                    return InteractionMode::Inspect;
                }
            };

            if transition.target.is_finished {
                params.commands.entity(target).remove::<Pending>();
                self.placement.finalize(target, params);
            }

            if transition.target.discontinue {
                params.cleanup();
                return InteractionMode::Inspect;
            }

            match self.continuity {
                SelectAnchorContinuity::ReplaceAnchor { .. } => {
                    // Backing out of ReplaceAnchor always means we go back to
                    // Inspect mode.
                    params.cleanup();
                    return InteractionMode::Inspect;
                }
                SelectAnchorContinuity::InsertElement => {
                    if transition.target.is_finished {
                        // If we have finished inserting an element then stop here
                        params.cleanup();
                        return InteractionMode::Inspect;
                    } else {
                        return InteractionMode::SelectAnchor(Self {
                            target: None,
                            placement: transition.placement.next,
                            continuity: SelectAnchorContinuity::InsertElement,
                            scope: self.scope,
                        });
                    }
                }
                SelectAnchorContinuity::Continuous { .. } => {
                    return InteractionMode::SelectAnchor(Self {
                        target: None,
                        placement: transition.placement.next,
                        continuity: SelectAnchorContinuity::Continuous { previous: None },
                        scope: self.scope,
                    });
                }
            }
        } else {
            // If there is no current target then a backout means we should
            // exit the SelectAnchor mode entirely.
            params.cleanup();
            return InteractionMode::Inspect;
        }
    }
}

#[derive(Clone)]
enum PlaceableObject {
    Model(Model),
    Anchor,
    VisualMesh(WorkcellModel),
    CollisionMesh(WorkcellModel),
}

#[derive(Clone)]
pub struct SelectAnchor3D {
    bundle: PlaceableObject,
    // Entity being edited
    target: Option<Entity>,
    // Proposed parent
    parent: Option<Entity>,
    // Continuity also stores the previous parent if needed
    continuity: SelectAnchorContinuity,
}

impl SelectAnchor3D {
    /// Create one new location. After an anchor is selected the new location
    /// will be created and the mode will return to Inspect.
    pub fn create_new_point() -> SelectAnchorPointBuilder {
        SelectAnchorPointBuilder {
            for_element: None,
            continuity: SelectAnchorContinuity::InsertElement,
        }
    }

    /// Move an existing location to a new anchor.
    pub fn replace_point(location: Entity, original_anchor: Entity) -> SelectAnchorPointBuilder {
        SelectAnchorPointBuilder {
            for_element: Some(location),
            continuity: SelectAnchorContinuity::ReplaceAnchor {
                original_anchor: Some(original_anchor),
            },
        }
    }

    /// Whether a new object is being created
    pub fn begin_creating(&self) -> bool {
        match self.continuity {
            SelectAnchorContinuity::ReplaceAnchor { .. } => false,
            SelectAnchorContinuity::InsertElement | SelectAnchorContinuity::Continuous { .. } => {
                self.target.is_none()
            }
        }
    }

    /// Always return none, 3D anchors are only selectable and we need to
    /// return to Inspect mode.
    fn next<'w, 's>(
        &self,
        _anchor_selection: AnchorSelection,
        _params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Self> {
        None
    }

    /// Used for updating parents on parent assignment
    fn update_parent<'w, 's>(
        &mut self,
        anchor_selection: AnchorSelection,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<(), ()> {
        if let Some(target) = self.target {
            // Make sure the selected entity is an anchor
            // TODO(luca) Should this be at the caller level?
            match params.anchors.get(anchor_selection.entity()) {
                Ok(anchor) => match anchor.1 {
                    Anchor::Pose3D(_) => {}
                    _ => return Err(()),
                },
                _ => return Err(()),
            }

            // Avoid endless loops by making sure the selected entity is not a child of the
            // current one
            for ancestor in AncestorIter::new(&params.parents, anchor_selection.entity()) {
                if ancestor == target {
                    return Err(());
                }
            }

            if self.parent != Some(anchor_selection.entity()) {
                match self.parent {
                    Some(_new_parent) => {
                        if anchor_selection.entity() != target {
                            self.parent = Some(anchor_selection.entity());
                        }
                        return Ok(());
                    }
                    None => {
                        error!("Reassigning parent for entity without a parent");
                        return Err(());
                    }
                }
            }
            return Err(());
        } else {
            error!("DEV error replacing anchor without original");
            return Err(());
        }
    }

    fn preview<'w, 's>(
        &mut self,
        anchor_selection: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> PreviewResult {
        // The only live update we need to do is on parent entity change
        if let SelectAnchorContinuity::ReplaceAnchor { original_anchor: _ } = self.continuity {
            match self.update_parent(AnchorSelection::Existing(anchor_selection), params) {
                Ok(()) => {
                    return PreviewResult::Updated3D(Self {
                        bundle: self.bundle.clone(),
                        parent: Some(anchor_selection),
                        target: self.target,
                        continuity: self.continuity,
                    });
                }
                Err(()) => {
                    return PreviewResult::Unchanged;
                }
            }
        }
        return PreviewResult::Unchanged;
    }

    pub fn backout<'w, 's>(
        &self,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> InteractionMode {
        params.cleanup();
        return InteractionMode::Inspect;
    }
}

enum PreviewResult {
    /// The SelectAnchor state needs to be updated
    Updated(SelectAnchor),
    /// The SelectAnchor3D state needs to be updated
    Updated3D(SelectAnchor3D),
    /// The SelectAnchor state is unchanged
    Unchanged,
    /// The SelectAnchor request was invalid and should exit
    Invalid,
}

pub fn handle_select_anchor_mode(
    mut mode: ResMut<InteractionMode>,
    anchors: Query<(), With<Anchor>>,
    transforms: Query<&GlobalTransform>,
    hovering: Res<Hovering>,
    mouse_button_input: Res<Input<MouseButton>>,
    touch_input: Res<Touches>,
    mut params: SelectAnchorPlacementParams,
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
    mut select: EventReader<Select>,
    mut hover: EventWriter<Hover>,
    blockers: Option<Res<PickingBlockers>>,
    workspace: Res<CurrentWorkspace>,
    open_sites: Query<Entity, With<NameOfSite>>,
    current_drawing: Res<CurrentEditDrawing>,
) {
    let mut request = match &*mode {
        InteractionMode::SelectAnchor(request) => request.clone(),
        _ => {
            return;
        }
    };

    if mode.is_changed() {
        // The mode was changed to this one on this update cycle. We should
        // check if something besides an anchor is being hovered, and clear it
        // out if it is.
        if let Some(hovering) = hovering.0 {
            if anchors.contains(hovering) {
                params
                    .cursor
                    .remove_mode(SELECT_ANCHOR_MODE_LABEL, &mut params.visibility);
            } else {
                hover.send(Hover(None));
                params
                    .cursor
                    .add_mode(SELECT_ANCHOR_MODE_LABEL, &mut params.visibility);
            }
        } else {
            params
                .cursor
                .add_mode(SELECT_ANCHOR_MODE_LABEL, &mut params.visibility);
        }

        // Make the anchor placement component of the cursor visible
        if request.site_scope() {
            set_visibility(
                params.cursor.site_anchor_placement,
                &mut params.visibility,
                true,
            );
        } else {
            set_visibility(
                params.cursor.level_anchor_placement,
                &mut params.visibility,
                true,
            );
        }

        match request.scope {
            Scope::General | Scope::Site => {
                // If we are working with normal level or site requests, hide all drawing anchors
                for anchor in params.anchors.iter().filter(|(e, _)| {
                    params
                        .parents
                        .get(*e)
                        .is_ok_and(|p| params.drawings.get(**p).is_ok())
                }) {
                    set_visibility(anchor.0, &mut params.visibility, false);
                    params.hidden_entities.drawing_anchors.insert(anchor.0);
                }
            }
            // Nothing to hide, it's done by the drawing editor plugin
            Scope::Drawing => {}
        }

        // If we are creating a new object, then we should deselect anything
        // that might be currently selected.
        if request.begin_creating() {
            if let Some(previous_selection) = selection.0 {
                if let Ok(mut selected) = selected.get_mut(previous_selection) {
                    selected.is_selected = false;
                }
                selection.0 = None;
            }
        }

        if request.continuity.needs_original() {
            // Keep track of the original anchor that we intend to replace so
            // that we can revert any previews.
            let for_element = match request.target {
                Some(for_element) => for_element,
                None => {
                    error!(
                        "for_element must be Some for ReplaceAnchor. \
                        Reverting to Inspect Mode."
                    );
                    params.cleanup();
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };

            let original = match request.placement.save_original(for_element, &mut params) {
                Some(original) => original,
                None => {
                    error!(
                        "cannot locate an original anchor for \
                        entity {:?}. Reverting to Inspect Mode.",
                        for_element,
                    );
                    params.cleanup();
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };

            request.continuity = SelectAnchorContinuity::ReplaceAnchor {
                original_anchor: Some(original),
            };
            // Save the new mode here in case it doesn't get saved by any
            // branches in the rest of this system function.
            *mode = InteractionMode::SelectAnchor(request.clone());
        }
    }

    if hovering.is_changed() {
        if hovering.0.is_none() {
            params
                .cursor
                .add_mode(SELECT_ANCHOR_MODE_LABEL, &mut params.visibility);
        } else {
            params
                .cursor
                .remove_mode(SELECT_ANCHOR_MODE_LABEL, &mut params.visibility);
        }
    }

    if select.is_empty() {
        let clicked = mouse_button_input.just_pressed(MouseButton::Left)
            || touch_input.iter_just_pressed().next().is_some();
        let blocked = blockers.filter(|x| x.blocking()).is_some();

        if clicked && !blocked {
            // Since the user clicked but there are no actual selections, the
            // user is effectively asking to create a new anchor at the current
            // cursor location. We will create that anchor and treat it as if it
            // were selected.
            let tf = match transforms.get(params.cursor.frame) {
                Ok(tf) => tf,
                Err(_) => {
                    error!(
                        "Could not get transform for cursor frame \
                        {:?} in SelectAnchor mode.",
                        params.cursor.frame,
                    );
                    // TODO(MXG): Put in backout behavior here.
                    return;
                }
            };

            let new_anchor = match request.scope {
                Scope::Site => {
                    let site = workspace.to_site(&open_sites).expect("No current site??");
                    let new_anchor = params.commands.spawn(AnchorBundle::at_transform(tf)).id();
                    params.commands.entity(site).add_child(new_anchor);
                    new_anchor
                }
                Scope::Drawing => {
                    let drawing_entity = current_drawing
                        .target()
                        .expect("No drawing while spawning drawing anchor")
                        .drawing;
                    let (parent, ppm) = params
                        .drawings
                        .get(drawing_entity)
                        .expect("Entity being edited is not a drawing");
                    // We also need to have a transform such that the anchor will spawn in the
                    // right spot
                    let pose = compute_parent_inverse_pose(&tf, &transforms, parent);
                    let ppm = ppm.0;
                    let new_anchor = params
                        .commands
                        .spawn(AnchorBundle::new([pose.trans[0], pose.trans[1]].into()))
                        .insert(Transform::from_scale(Vec3::new(ppm, ppm, 1.0)))
                        .set_parent(parent)
                        .id();
                    new_anchor
                }
                Scope::General => params.commands.spawn(AnchorBundle::at_transform(tf)).id(),
            };

            request = match request.next(AnchorSelection::new(new_anchor), &mut params) {
                Some(next_mode) => next_mode,
                None => {
                    params.cleanup();
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };

            *mode = InteractionMode::SelectAnchor(request);
        } else {
            // Offer a preview based on the current hovering status
            let hovered = hovering.0.unwrap_or(params.cursor.level_anchor_placement);
            let current = request
                .target
                .map(|target| request.placement.current(target, &params))
                .flatten();

            if Some(hovered) != current {
                // We should only call this function if the current hovered
                // anchor is not the one currently assigned. Otherwise we
                // are wasting query+command effort.
                match request.preview(hovered, &mut params) {
                    PreviewResult::Updated(next) => {
                        *mode = InteractionMode::SelectAnchor(next);
                    }
                    PreviewResult::Updated3D(next) => {
                        *mode = InteractionMode::SelectAnchor3D(next);
                    }
                    PreviewResult::Unchanged => {
                        // Do nothing, the mode has not changed
                    }
                    PreviewResult::Invalid => {
                        // Something was invalid about the request, so we
                        // will exit back to Inspect mode.
                        params.cleanup();
                        *mode = InteractionMode::Inspect;
                    }
                };
            }
        }
    } else {
        for new_selection in select
            .read()
            .filter_map(|s| s.0)
            .filter(|s| anchors.contains(*s))
        {
            request = match request.next(AnchorSelection::existing(new_selection), &mut params) {
                Some(next_mode) => next_mode,
                None => {
                    params.cleanup();
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };
        }

        *mode = InteractionMode::SelectAnchor(request);
    }
}

fn compute_parent_inverse_pose(
    tf: &GlobalTransform,
    transforms: &Query<&GlobalTransform>,
    parent: Entity,
) -> Pose {
    let parent_tf = transforms
        .get(parent)
        .expect("Failed in fetching parent transform");

    let inv_tf = parent_tf.affine().inverse();
    let goal_tf = tf.affine();
    let mut pose = Pose::default();
    pose.rot = pose.rot.as_euler_extrinsic_xyz();
    pose.align_with(&Transform::from_matrix((inv_tf * goal_tf).into()))
}

pub fn handle_select_anchor_3d_mode(
    mut mode: ResMut<InteractionMode>,
    anchors: Query<(), With<Anchor>>,
    transforms: Query<&GlobalTransform>,
    hovering: Res<Hovering>,
    mouse_button_input: Res<Input<MouseButton>>,
    touch_input: Res<Touches>,
    mut params: SelectAnchorPlacementParams,
    selection: Res<Selection>,
    mut select: EventReader<Select>,
    mut hover: EventWriter<Hover>,
    blockers: Option<Res<PickingBlockers>>,
    workspace: Res<CurrentWorkspace>,
    app_state: Res<State<AppState>>,
) {
    let mut request = match &*mode {
        InteractionMode::SelectAnchor3D(request) => request.clone(),
        _ => {
            return;
        }
    };

    if mode.is_changed() {
        // The mode was changed to this one on this update cycle. We should
        // check if something besides an anchor is being hovered, and clear it
        // out if it is.
        if let Some(hovering) = hovering.0 {
            if anchors.contains(hovering) {
                params
                    .cursor
                    .remove_mode(SELECT_ANCHOR_MODE_LABEL, &mut params.visibility);
            } else {
                hover.send(Hover(None));
                params
                    .cursor
                    .add_mode(SELECT_ANCHOR_MODE_LABEL, &mut params.visibility);
            }
        } else {
            params
                .cursor
                .add_mode(SELECT_ANCHOR_MODE_LABEL, &mut params.visibility);
        }

        // Make the anchor placement component of the cursor visible
        match request.bundle {
            PlaceableObject::Anchor => {
                set_visibility(params.cursor.frame_placement, &mut params.visibility, true);
            }
            PlaceableObject::Model(ref m) => {
                // Spawn the model as a child of the cursor
                params
                    .cursor
                    .set_model_preview(&mut params.commands, Some(m.clone()));
            }
            PlaceableObject::VisualMesh(ref m) | PlaceableObject::CollisionMesh(ref m) => {
                // Spawn the model as a child of the cursor
                params
                    .cursor
                    .set_workcell_model_preview(&mut params.commands, Some(m.clone()));
            }
        }

        // Set the request parent to the currently selected element, to spawn new object as
        // children of selected frames
        if matches!(**app_state, AppState::WorkcellEditor) && request.begin_creating() {
            request.parent = selection.0;
        }
    }

    if select.is_empty() {
        let clicked = mouse_button_input.just_pressed(MouseButton::Left)
            || touch_input.iter_just_pressed().next().is_some();
        let blocked = blockers.filter(|x| x.blocking()).is_some();

        if clicked && !blocked {
            if request.begin_creating() {
                // Since the user clicked but there are no actual selections, the
                // user is effectively asking to create a new anchor at the current
                // cursor location. We will create that anchor and treat it as if it
                // were selected.
                let cursor_tf = transforms
                    .get(params.cursor.frame)
                    .expect("Unable to get transform for cursor frame");

                let parent = request
                    .parent
                    .unwrap_or(workspace.root.expect("No workspace"));
                let pose = compute_parent_inverse_pose(&cursor_tf, &transforms, parent);
                let id = match request.bundle {
                    PlaceableObject::Anchor => params
                        .commands
                        .spawn((
                            AnchorBundle::new(Anchor::Pose3D(pose)),
                            FrameMarker,
                            NameInWorkcell("Unnamed".to_string()),
                        ))
                        .id(),
                    PlaceableObject::Model(ref a) => {
                        let mut model = a.clone();
                        // If we are in workcell mode, add a "base link" frame to the model
                        let child_id = params.commands.spawn_empty().id();
                        if matches!(**app_state, AppState::WorkcellEditor) {
                            params.commands.spawn_model(child_id, model, None);
                            params
                                .commands
                                .spawn((
                                    AnchorBundle::new(Anchor::Pose3D(pose))
                                        .dependents(Dependents::single(child_id)),
                                    FrameMarker,
                                    NameInWorkcell("model_root".to_string()),
                                ))
                                .add_child(child_id)
                                .id()
                        } else {
                            model.pose = pose;
                            params.commands.spawn_model(child_id, model, None);
                            child_id
                        }
                    }
                    PlaceableObject::VisualMesh(ref a) => {
                        let mut model = a.clone();
                        model.pose = pose;
                        let mut cmd = params.commands.spawn(VisualMeshMarker);
                        model.add_bevy_components(&mut cmd);
                        cmd.id()
                    }
                    PlaceableObject::CollisionMesh(ref a) => {
                        let mut model = a.clone();
                        model.pose = pose;
                        let mut cmd = params.commands.spawn(CollisionMeshMarker);
                        model.add_bevy_components(&mut cmd);
                        cmd.id()
                    }
                };
                // Add child and dependent to parent
                params.commands.entity(id).set_parent(parent);
                if let Ok(mut deps) = params.dependents.get_mut(parent) {
                    deps.insert(id);
                }
            } else {
                // We are replacing an anchor, which in this mode refers to changing a parent
                if let (Some(target), Some(parent)) = (request.target, request.parent) {
                    if let Ok(old_parent) = params.parents.get(target) {
                        if let Ok(mut deps) = params.dependents.get_mut(**old_parent) {
                            deps.remove(&target);
                        }
                    }
                    if let Ok(mut deps) = params.dependents.get_mut(parent) {
                        deps.insert(target);
                    }
                    let mut cmd = params.commands.entity(target);
                    cmd.set_parent(parent);
                    // Anchors store their pose in the Anchor component, other elements in Pose,
                    // set accordingly
                    let previous_tf = transforms
                        .get(target)
                        .expect("Transform not found for entity");
                    let pose = compute_parent_inverse_pose(&previous_tf, &transforms, parent);
                    if anchors.get(target).is_ok() {
                        cmd.insert(AnchorBundle::new(Anchor::Pose3D(pose)));
                    } else {
                        cmd.insert(pose);
                    }
                }
            }

            params.cleanup();
            *mode = InteractionMode::Inspect;
            return;
        } else {
            // Offer a preview based on the current hovering status
            let hovered = hovering.0.unwrap_or(params.cursor.frame_placement);
            let current = request.parent;

            if Some(hovered) != current {
                // We should only call this function if the current hovered
                // anchor is not the one currently assigned. Otherwise we
                // are wasting query+command effort.
                match request.preview(hovered, &mut params) {
                    PreviewResult::Updated(_next) => {
                        // We should never get here
                        unreachable!();
                    }
                    PreviewResult::Updated3D(next) => {
                        *mode = InteractionMode::SelectAnchor3D(next);
                    }
                    PreviewResult::Unchanged => {
                        // Do nothing, the mode has not changed
                    }
                    PreviewResult::Invalid => {
                        // Something was invalid about the request, so we
                        // will exit back to Inspect mode.
                        params.cleanup();
                        *mode = InteractionMode::Inspect;
                    }
                };
            }
        }
    } else {
        for new_selection in select
            .read()
            .filter_map(|s| s.0)
            .filter(|s| anchors.contains(*s))
        {
            request = match request.next(AnchorSelection::existing(new_selection), &mut params) {
                Some(next_mode) => next_mode,
                None => {
                    params.cleanup();
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };
        }
    }
    *mode = InteractionMode::SelectAnchor3D(request);
}

impl From<SelectAnchor> for InteractionMode {
    fn from(mode: SelectAnchor) -> Self {
        InteractionMode::SelectAnchor(mode)
    }
}

impl From<SelectAnchor3D> for InteractionMode {
    fn from(mode: SelectAnchor3D) -> Self {
        InteractionMode::SelectAnchor3D(mode)
    }
}
