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
    site::{Anchor, AnchorBundle, AnchorDependents, Pending, PathBehavior, Original},
};
use rmf_site_format::{
    Side, Edge, Point, Path, Lane, Measurement, Wall, Door, LiftProperties, Location, Floor,
};
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use std::sync::Arc;

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
    ReplaceAnchor{original_anchor: Option<Entity>},
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
    /// Backout behavior is the same as InsertElement, except any elements that
    /// finished being created along the way will remain in tact while backing
    /// out.
    Continuous,
}

impl SelectAnchorContinuity {
    fn needs_original(&self) -> bool {
        match self {
            Self::ReplaceAnchor{original_anchor} => {
                return original_anchor.is_none();
            },
            _ => { return false; }
        }
    }

    fn replacing(&self) -> Option<Entity> {
        match self {
            Self::ReplaceAnchor{original_anchor} => *original_anchor,
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
        Self{
            target: input.0,
            placement: input.1,
        }
    }
}

type CreateEdgeFn = Arc<dyn Fn(&mut SelectAnchorPlacementParams, Edge<Entity>) -> Entity + Send + Sync>;
type CreatePointFn = Arc<dyn Fn(&mut SelectAnchorPlacementParams, Point<Entity>) -> Entity + Send + Sync>;
type CreatePathFn = Arc<dyn Fn(&mut SelectAnchorPlacementParams, Path<Entity>) -> Entity + Send + Sync>;
type FinalizeFn = Arc<dyn Fn(&mut SelectAnchorPlacementParams, Entity)  + Send + Sync>;

struct TargetTransition {
    created: Option<Entity>,
    is_finished: Option<FinalizeFn>,
}

impl TargetTransition {
    fn none() -> Self {
        Self{
            created: None,
            is_finished: None,
        }
    }

    fn create(e: Entity) -> Self {
        Self{
            created: Some(e),
            is_finished: None,
        }
    }

    fn finished(finalize: FinalizeFn) -> Self {
        Self{
            created: None,
            is_finished: Some(finalize),
        }
    }

    fn finish(mut self, finalize: FinalizeFn) -> Self {
        self.is_finished = Some(finalize);
        self
    }

    fn current(&self, e: Option<Entity>) -> Option<Entity> {
        match e {
            Some(e) => {
                if self.created.is_some() {
                    println!(
                        "DEV ERROR: Created a superfluous target while in \
                        SelectAnchor mode"
                    );
                }
                Some(e)
            },
            None => {
                match self.created {
                    Some(e) => Some(e),
                    None => {
                        println!(
                            "DEV ERROR: Failed to create an entity while in \
                            SelectAnchor mode"
                        );
                        None
                    }
                }
            }
        }
    }

    fn next(&self, e: Option<Entity>) -> Option<Entity> {
        if self.is_finished.is_some() {
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
        anchor_selection: Entity,
        replacing: Option<Entity>,
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
}

#[derive(Clone)]
pub struct EdgePlacement {
    side: Side,
    create: CreateEdgeFn,
    finalize: FinalizeFn,
}

impl EdgePlacement {
    fn transition(&self) -> PlacementTransition {
        PlacementTransition{
            preview: None,
            next: Arc::new(Self{
                side: self.side.opposite(),
                create: self.create.clone(),
                finalize: self.finalize.clone(),
            }),
        }
    }

    fn ignore(&self) -> PlacementTransition {
        PlacementTransition{
            preview: None,
            next: Arc::new(self.clone()),
        }
    }

    fn new<T: Bundle + From<Edge<Entity>>>(side: Side) -> Arc<Self> {
        Arc::new(Self{
            side,
            create: Arc::new(
                |params: &mut SelectAnchorPlacementParams, edge: Edge<Entity>| {
                    let new_bundle: T = edge.into();
                    params.commands
                        .spawn()
                        .insert_bundle(new_bundle)
                        .insert(Pending)
                        .id()
                }
            ),
            finalize: Arc::new(
                |params: &mut SelectAnchorPlacementParams, entity: Entity| {
                    dbg!("Removing Original edge");
                    params.commands.entity(entity).remove::<Original<Edge<Entity>>>();
                }
            )
        })
    }
}

impl Placement for EdgePlacement {
    fn next<'w, 's>(
        &self,
        anchor_selection: Entity,
        replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()> {
        let (target, mut endpoints, original) = match target {
            Some(target) => {
                // We expect that we already have an element and we are
                // modifying it.
                match params.edges.get_mut(target) {
                    Ok((edge, original)) => {
                        (target, edge, original)
                    },
                    Err(_) => {
                        println!(
                            "DEV ERROR: Entity {:?} is not the right kind of \
                            element", target,
                        );
                        return Err(());
                    }
                }
            },
            None => {
                // We need to begin creating a new element
                let anchors = Edge::new(
                    anchor_selection, params.cursor.anchor_placement
                );
                let target = (*self.create)(params, anchors);
                let mut deps = match params.dependents.get_many_mut(anchors.array()) {
                    Ok(deps) => deps,
                    Err(_) => {
                        // One of the anchors was not a valid anchor, so we
                        // should abort.
                        println!(
                            "DEV ERROR: Invalid anchors being selected: {:?} and {:?}",
                            anchors.left(), anchors.right(),
                        );
                        return Err(());
                    }
                };

                for dep in &mut deps {
                    dep.dependents.insert(target);
                }
                return Ok((TargetTransition::create(target), self.transition()).into());
            }
        };

        println!("=================================");
        let new_edge = match original {
            Some(original) => {
                if anchor_selection == original.side(self.side.opposite()) {
                    // The user is asking to swap the anchors
                    original.in_reverse()
                } else {
                    original.with_side_of(self.side, anchor_selection)
                }
            },
            None => {
                match replacing {
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

                        new_edge.with_side_of(self.side, anchor_selection)
                    },
                    None => {
                        if endpoints.side(self.side.opposite()) == anchor_selection {
                            // The user is asking to select the same anchor for
                            // both sides of the edge. This is not okay, so we
                            // ignore this request.
                            return Ok((TargetTransition::none(), self.ignore()).into());
                        }

                        endpoints.with_side_of(self.side, anchor_selection)
                    }
                }
            }
        };

        // Remove the target edge as a dependency from any anchors that are no
        // longer being used by this edge.
        for old_anchor in endpoints.array() {
            if new_edge.array().iter().find(|x| **x == old_anchor).is_none() {
                // This anchor is no longer being used by the edge.
                if let Ok(mut deps) = params.dependents.get_mut(old_anchor) {
                    deps.dependents.remove(&target);
                } else {
                    println!(
                        "DEV ERROR: No AnchorDependents component found for \
                        {:?} while in SelectAnchor mode.", old_anchor
                    );
                }
            }
        }

        for new_anchor in new_edge.array() {
            if endpoints.array().iter().find(|x| **x == new_anchor).is_none() {
                // This anchor was not being used by the edge previously.
                if let Ok(mut deps) = params.dependents.get_mut(new_anchor) {
                    deps.dependents.insert(target);
                } else {
                    println!(
                        "DEV ERROR: No AnchorDependents component found for \
                        {:?} while in SelectAnchor mode.", new_anchor
                    );
                }
            }
        }

        *endpoints = new_edge;
        return match self.side {
            Side::Left => Ok((TargetTransition::none(), self.transition()).into()),
            Side::Right => Ok((TargetTransition::finished(self.finalize.clone()), self.transition()).into()),
        };
    }

    fn current<'w, 's>(
        &self,
        target: Entity,
        params: &SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> {
        params.edges.get(target).ok().map(|edge| edge.0.side(self.side))
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
}

#[derive(Clone)]
pub struct PointPlacement {
    create: CreatePointFn,
    finalize: FinalizeFn,
}

impl PointPlacement {
    fn new<T: Bundle + From<Point<Entity>>>() -> Arc<Self> {
        Arc::new(Self{
            create: Arc::new(
                |params: &mut SelectAnchorPlacementParams, point: Point<Entity>| {
                    let new_bundle: T = point.into();
                    params.commands
                        .spawn()
                        .insert_bundle(new_bundle)
                        .insert(Pending)
                        .id()
                }
            ),
            finalize: Arc::new(
                |params: &mut SelectAnchorPlacementParams, entity: Entity| {
                    params.commands.entity(entity).remove::<Original<Point<Entity>>>();
                }
            )
        })
    }
}

impl PointPlacement {
    fn transition(&self) -> PlacementTransition {
        PlacementTransition{
            preview: None,
            next: Arc::new(self.clone()),
        }
    }
}

impl Placement for PointPlacement {
    fn next<'w, 's>(
        &self,
        anchor_selection: Entity,
        _replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()> {
        match target {
            Some(target) => {
                // Change the anchor that the location is attached to.
                let point = match params.points.get_mut(target) {
                    Ok(l) => l,
                    Err(_) => {
                        println!(
                            "DEV ERROR: Unable to get location {:?} while in \
                            SelectAnchor mode.", target
                        );
                        return Err(());
                    }
                };

                if **point != anchor_selection {
                    match params.dependents.get_many_mut(
                        [**point, anchor_selection]
                    ) {
                        Ok([mut old_dep, mut new_dep]) => {
                            old_dep.dependents.remove(&target);
                            new_dep.dependents.insert(target);
                        },
                        Err(_) => {
                            println!(
                                "DEV ERROR: Unable to get anchor dependents \
                                for [{:?}, {:?}] while in SelectAnchor mode.",
                                point,
                                anchor_selection,
                            );
                            return Err(());
                        }
                    }
                }

                return Ok((TargetTransition::finished(self.finalize.clone()), self.transition()).into());
            }
            None => {
                // The element doesn't exist yet, so we need to spawn one.
                let target = (*self.create)(params, Point(anchor_selection));
                if let Ok(mut dep) = params.dependents.get_mut(anchor_selection) {
                    dep.dependents.insert(target);
                } else {
                    println!("DEV ERROR: Unable to get anchor dependents for {anchor_selection:?}");
                }

                return Ok((TargetTransition::create(target).finish(self.finalize.clone()), self.transition()).into());
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
}

#[derive(Clone)]
pub struct PathPlacement {
    /// Replace the floor anchor at the specified index, or push the anchor to
    /// the end if None is specified. If the specified index is too high, this
    /// value will be changed to None and all new anchors will be pushed to the
    /// back.
    placement: Option<usize>,
    create: CreatePathFn,
    finalize: FinalizeFn,
}

impl PathPlacement {

    fn new<T: Bundle + From<Path<Entity>>>(placement: Option<usize>) -> Arc<Self> {
        Arc::new(Self{
            placement,
            create: Arc::new(
                |params: &mut SelectAnchorPlacementParams, path: Path<Entity>| {
                    let new_bundle: T = path.into();
                    params.commands
                        .spawn()
                        .insert_bundle(new_bundle)
                        .insert(Pending)
                        .id()
                }
            ),
            finalize: Arc::new(
                |params: &mut SelectAnchorPlacementParams, entity: Entity| {
                    params.commands.entity(entity).remove::<Original<Path<Entity>>>();
                }
            )
        })
    }

    fn at_index(&self, index: usize) -> Arc<Self> {
        Arc::new(Self{
            placement: Some(index),
            create: self.create.clone(),
            finalize: self.finalize.clone(),
        })
    }

    fn transition_from(&self, index: usize) -> PlacementTransition {
        PlacementTransition{
            preview: Some(self.at_index(index)),
            next: self.at_index(index+1),
        }
    }

    fn transition_to(&self, index: usize) -> PlacementTransition {
        let index = if index > 0 { index - 1 } else { 0 };
        self.transition_from(index)
    }
}

impl Placement for PathPlacement {
    fn next<'w, 's>(
        &self,
        anchor_selection: Entity,
        _replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition, ()> {
        let target = match target {
            Some(target) => target,
            None => {
                // We need to create a new element
                let target = (*self.create)(params, Path(vec![anchor_selection]));

                match params.dependents.get_mut(anchor_selection) {
                    Ok(mut dep) => {
                        dep.dependents.insert(target);
                    },
                    Err(_) => {
                        println!(
                            "DEV ERROR: Invalid anchor being selected",
                        );
                    }
                }

                return Ok((TargetTransition::create(target), self.transition_from(0)).into());
            }
        };

        let (mut path, behavior) = match params.paths.get_mut(target) {
            Ok(q) => q,
            Err(_) => {
                println!(
                    "DEV ERROR: Unable to find path info for {target:?} while \
                    in SelectAnchor mode."
                );
                return Err(());
            }
        };

        let index = self.placement.unwrap_or(path.len()).min(path.len());
        if path.len() >= behavior.minimum_points && behavior.implied_complete_loop {
            if Some(anchor_selection) == path.first().cloned() {
                if index >= path.len() - 1 {
                    // The user has set the first node to the last node,
                    // creating a closed loop. We should consider the floor to
                    // be finished.
                    if index == path.len() - 1 {
                        // Remove the last element because it is redundant with
                        // the first element now.
                        path.pop();
                    }
                    return Ok((
                        TargetTransition::finished(self.finalize.clone()),
                        self.transition_from(index)
                    ).into());
                }
            }
        }

        if !behavior.allow_inner_loops {
            for (i, anchor) in path.iter().enumerate() {
                if *anchor == anchor_selection && i != index {
                    // The user has reselected a midpoint. That violates the
                    // requested behavior, so we ignore it.
                    return Ok((TargetTransition::none(), self.transition_to(index)).into());
                }
            }
        }

        if let Some(place_anchor) = path.get_mut(index) {
            let old_anchor = *place_anchor;
            *place_anchor = anchor_selection;

            if path.iter().find(|x| **x == old_anchor).is_none() {
                if let Ok(mut dep) = params.dependents.get_mut(old_anchor) {
                    // Remove the dependency for the old anchor since we are not
                    // using it anymore.
                    dep.dependents.remove(&target);
                } else {
                    println!(
                        "DEV ERROR: Invalid old anchor {:?} in path", old_anchor
                    );
                }
            } else {
                println!(
                    "DEV ERROR: Anchor {old_anchor:?} was duplicated in a path"
                );
            }
        } else {
            // We need to add this anchor to the end of the vector
            path.push(anchor_selection);
        }

        let mut dep = match params.dependents.get_mut(anchor_selection) {
            Ok(dep) => dep,
            Err(_) => {
                println!(
                    "DEV ERROR: Invalid anchor being selected",
                );
                return Ok((TargetTransition::none(), self.transition_to(index)).into());
            }
        };
        dep.dependents.insert(target);

        return Ok((TargetTransition::none(), self.transition_from(index)).into());
    }

    fn current<'w, 's>(
        &self,
        target: Entity,
        params: &SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> {
        let index = match self.placement {
            Some(i) => i,
            None => { return None; }
        };

        let path = match params.paths.get(target) {
            Ok(p) => p.0,
            Err(_) => {
                println!(
                    "DEV ERROR: Unable to find path for {:?} while in \
                    SelectAnchor mode", target,
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
                println!(
                    "DEV ERROR: Unable to find path for {:?} while in \
                    SelectAnchor mode", target,
                );
                return None;
            }
        };

        let placement = self.placement.map(|index| path.get(index).cloned()).flatten();
        params.commands.entity(target).insert(Original(path));
        return placement;
    }
}

#[derive(SystemParam)]
pub struct SelectAnchorPlacementParams<'w, 's> {
    edges: Query<'w, 's, (&'static mut Edge<Entity>, Option<&'static Original<Edge<Entity>>>)>,
    points: Query<'w, 's, &'static mut Point<Entity>>,
    paths: Query<'w, 's, (&'static mut Path<Entity>, &'static PathBehavior)>,
    dependents: Query<'w, 's, &'static mut AnchorDependents>,
    commands: Commands<'w, 's>,
    cursor: Res<'w, Cursor>,
}

pub struct SelectAnchorEdgeBuilder {
    for_element: Option<Entity>,
    placement: Side,
    continuity: SelectAnchorContinuity,
}

impl SelectAnchorEdgeBuilder {
    pub fn for_lane(self) -> SelectAnchor {
        SelectAnchor{
            target: self.for_element,
            placement: EdgePlacement::new::<Lane<Entity>>(self.placement),
            continuity: self.continuity,
        }
    }

    pub fn for_measurement(self) -> SelectAnchor {
        SelectAnchor{
            target: self.for_element,
            placement: EdgePlacement::new::<Measurement<Entity>>(self.placement),
            continuity: self.continuity,
        }
    }

    pub fn for_wall(self) -> SelectAnchor {
        SelectAnchor{
            target: self.for_element,
            placement: EdgePlacement::new::<Wall<Entity>>(self.placement),
            continuity: self.continuity,
        }
    }

    pub fn for_door(self) -> SelectAnchor {
        SelectAnchor{
            target: self.for_element,
            placement: EdgePlacement::new::<Door<Entity>>(self.placement),
            continuity: self.continuity,
        }
    }

    pub fn for_lift(self) -> SelectAnchor {
        SelectAnchor{
            target: self.for_element,
            placement: EdgePlacement::new::<LiftProperties<Entity>>(self.placement),
            continuity: self.continuity,
        }
    }
}

pub struct SelectAnchorPointBuilder {
    for_element: Option<Entity>,
    continuity: SelectAnchorContinuity,
}

impl SelectAnchorPointBuilder {
    pub fn for_location(self) -> SelectAnchor {
        SelectAnchor{
            target: self.for_element,
            placement: PointPlacement::new::<Location<Entity>>(),
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
        SelectAnchor{
            target: self.for_element,
            placement: PathPlacement::new::<Floor<Entity>>(self.placement),
            continuity: self.continuity,
        }
    }
}

type PlacementArc = Arc<dyn Placement + Send + Sync>;

/// This enum requests that the next selection should be an anchor, and that
/// selection should be provided to one of the enumerated entities. When the
/// inner object is None, that means the selection action should create a new
/// instance of one.
#[derive(Clone)]
pub struct SelectAnchor {
    target: Option<Entity>,
    placement: PlacementArc,
    continuity: SelectAnchorContinuity,
}

impl SelectAnchor {

    pub fn replace_side(
        edge: Entity,
        side: Side,
    ) -> SelectAnchorEdgeBuilder {
        SelectAnchorEdgeBuilder{
            for_element: Some(edge),
            placement: side,
            continuity: SelectAnchorContinuity::ReplaceAnchor{
                original_anchor: None
            },
        }
    }

    /// Create a single new element of some edge-like type, e.g. Lane, Wall.
    ///
    /// # Examples
    ///
    /// ```
    /// let mode = SelectAnchor::create_one_new_edge().for_lane();
    /// ```
    pub fn create_one_new_edge() -> SelectAnchorEdgeBuilder {
        SelectAnchorEdgeBuilder{
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
    /// ```
    /// let mode = SelectAnchor::create_new_path().for_wall();
    /// ```
    pub fn create_new_edge_sequence() -> SelectAnchorEdgeBuilder {
        SelectAnchorEdgeBuilder{
            for_element: None,
            placement: Side::Left,
            continuity: SelectAnchorContinuity::Continuous,
        }
    }

    /// Create one new location. After an anchor is selected the new location
    /// will be created and the mode will return to Inspect.
    pub fn create_new_point() -> SelectAnchorPointBuilder {
        SelectAnchorPointBuilder{
            for_element: None,
            continuity: SelectAnchorContinuity::InsertElement,
        }
    }

    /// Move an existing location to a new anchor.
    pub fn replace_point(
        location: Entity,
        original_anchor: Entity,
    ) -> SelectAnchorPointBuilder {
        SelectAnchorPointBuilder{
            for_element: Some(location),
            continuity: SelectAnchorContinuity::ReplaceAnchor{
                original_anchor: None,
            }
        }
    }

    /// Create a new floor. The user will be able to select anchors continuously
    /// until they Backout. If the user selects an anchor that is already part
    /// of the floor the selection will be ignored, unless it is the first
    /// anchor of the floor, in which case a Backout will occur.
    pub fn create_new_path() -> SelectAnchorPathBuilder {
        SelectAnchorPathBuilder{
            for_element: None,
            placement: None,
            continuity: SelectAnchorContinuity::Continuous,
        }
    }

    /// Replace which anchor one of the points on the floor is using.
    pub fn replace_path_point(
        path: Entity,
        index: usize,
    ) -> SelectAnchorPathBuilder {
        SelectAnchorPathBuilder{
            for_element:  Some(path),
            placement: Some(index),
            continuity: SelectAnchorContinuity::ReplaceAnchor{
                original_anchor: None,
            },
        }
    }

    pub fn extend_path(path: Entity) -> SelectAnchorPathBuilder {
        SelectAnchorPathBuilder{
            for_element: Some(path),
            placement: None,
            continuity: SelectAnchorContinuity::InsertElement,
        }
    }

    /// Whether a new object is being created
    pub fn begin_creating(&self) -> bool {
        match self.continuity {
            SelectAnchorContinuity::ReplaceAnchor{ .. } => false,
            SelectAnchorContinuity::InsertElement
            | SelectAnchorContinuity::Continuous => self.target.is_none(),
        }
    }

    /// Get what the next mode should be if an anchor is selected during the
    /// current mode. If None is returned, that means we are done selecting
    /// anchors and should return to Inspect mode.
    fn next<'w, 's>(
        &self,
        anchor_selection: Entity,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Self> {
        let transition = match self.placement.next(
            anchor_selection,
            self.continuity.replacing(),
            self.target,
            params,
        ) {
            Ok(t) => t,
            Err(_) => { return None; }
        };

        if let Some(finalize) = &transition.target.is_finished {
            if let Some(finished_target) = transition.target.current(self.target) {
                // Remove the Pending marker from the target because it has
                // been finished.
                params.commands.entity(finished_target).remove::<Pending>();
                finalize(params, finished_target);
            } else {
                println!(
                    "DEV ERROR: An element was supposed to be finished by \
                    SelectAnchor, but we could not find it"
                );
            }
        }

        let next_target = transition.target.next(self.target);
        let next_placement = transition.placement.next;

        match self.continuity {
            SelectAnchorContinuity::ReplaceAnchor{..} => {
                // No matter what gets returned for next_target or next_placement
                // we exit the ReplaceAnchor mode as soon as a selection is made.
                return None;
            },
            SelectAnchorContinuity::InsertElement => {
                match next_target {
                    Some(next_target) => {
                        return Some(Self{
                            target: Some(next_target),
                            placement: next_placement,
                            continuity: self.continuity,
                        });
                    },
                    None => {
                        // If the next target is none, then the last
                        // selection finished constructing an element.
                        // Since this is InsertElement mode, we should
                        // quit here.
                        return None;
                    }
                }
            },
            SelectAnchorContinuity::Continuous => {
                return Some(Self{
                    target: next_target,
                    placement: next_placement,
                    continuity: self.continuity,
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
            anchor_selection,
            self.continuity.replacing(),
            self.target,
            params,
        ) {
            Ok(t) => t,
            Err(_) => { return PreviewResult::Invalid; }
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
            return PreviewResult::Updated(Self{
                target: Some(target),
                placement: new_placement.clone(),
                continuity: self.continuity
            });
        }

        if Some(target) == self.target {
            // Neither the placement nor the target has changed due to this
            // preview, so just return the Unchanged variant.
            return PreviewResult::Unchanged;
        }

        return PreviewResult::Updated(Self{
            target: Some(target),
            placement: self.placement.clone(),
            continuity: self.continuity
        });
    }
}

enum PreviewResult {
    /// The SelectAnchor state needs to be updated
    Updated(SelectAnchor),
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
    mut visibility: Query<&mut Visibility>,
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
    mut select: EventReader<Select>,
    mut hover: EventWriter<Hover>,
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
            if !anchors.contains(hovering) {
                hover.send(Hover(None));
            }
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

        dbg!("Checking if we need an original");
        if request.continuity.needs_original() {
            dbg!("Yes we need an original");
            // Keep track of the original anchor that we intend to replace so
            // that we can revert any previews.
            let for_element = match request.target {
                Some(for_element) => for_element,
                None => {
                    println!(
                        "DEV ERROR: for_element must be Some for ReplaceAnchor. \
                        Reverting to Inspect Mode."
                    );
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };

            let original = match request.placement.save_original(for_element, &mut params) {
                Some(original) => original,
                None => {
                    println!(
                        "DEV ERROR: cannot locate an original anchor for \
                        entity {:?}. Reverting to Inspect Mode.",
                        for_element,
                    );
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };

            request.continuity = SelectAnchorContinuity::ReplaceAnchor{
                original_anchor: Some(original)
            };
            // Save the new mode here in case it doesn't get saved by any
            // branches in the rest of this system function.
            *mode = InteractionMode::SelectAnchor(request.clone());
        }
    }

    if hovering.is_changed() {
        dbg!();
        if hovering.0.is_none() {
            set_visibility(params.cursor.frame, &mut visibility, true);
            set_visibility(params.cursor.anchor_placement, &mut visibility, true);
        } else {
            set_visibility(params.cursor.frame, &mut visibility, false);
            set_visibility(params.cursor.anchor_placement, &mut visibility, false);
        }
    }

    if select.is_empty() {
        let clicked = mouse_button_input.just_pressed(MouseButton::Left)
            || touch_input.iter_just_pressed().next().is_some();

        if clicked {
            // Since the user clicked but there are no actual selections, the
            // user is effectively asking to create a new anchor at the current
            // cursor location. We will create that anchor and treat it as if it
            // were selected.
            let tf = match transforms.get(params.cursor.frame) {
                Ok(tf) => tf,
                Err(_) => {
                    println!(
                        "DEV ERROR: Could not get transform for cursor frame \
                        {:?} in SelectAnchor mode.",
                        params.cursor.frame,
                    );
                    // TODO(MXG): Put in backout behavior here.
                    return;
                }
            };

            let new_anchor = params.commands
                .spawn_bundle(AnchorBundle::at_transform(tf))
                .id();

            request = match request.next(new_anchor, &mut params) {
                Some(next_mode) => next_mode,
                None => {
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };

            *mode = InteractionMode::SelectAnchor(request);
        } else {
            if let Some(target) = request.target {
                // Offer a preview based on the current hovering status
                let hovered = hovering.0.unwrap_or(params.cursor.anchor_placement);
                let current = request.placement.current(target, &params);

                if Some(hovered) != current {
                    // We should only call this function if the current hovered
                    // anchor is not the one currently assigned. Otherwise we
                    // are wasting query+command effort.
                    match request.preview(hovered, &mut params) {
                        PreviewResult::Updated(next) => {
                            *mode = InteractionMode::SelectAnchor(next);
                        },
                        PreviewResult::Unchanged => {
                            // Do nothing, the mode has not changed
                        },
                        PreviewResult::Invalid => {
                            // Something was invalid about the request, so we
                            // will exit back to Inspect mode.
                            *mode = InteractionMode::Inspect;
                        }
                    };
                }
            }
        }
    } else {
        for new_selection in select.iter()
            .filter_map(|s| s.0)
            .filter(|s| anchors.contains(*s))
        {
            request = match request.next(new_selection, &mut params) {
                Some(next_mode) => next_mode,
                None => {
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };
        }

        *mode = InteractionMode::SelectAnchor(request);
    }
}
