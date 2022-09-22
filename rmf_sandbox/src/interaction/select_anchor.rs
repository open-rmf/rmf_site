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
    site::{Anchor, AnchorBundle, AnchorDependents, Pending, PathBehavior},
};
use rmf_site_format::{
    Side, Edge, Point, Path, Lane, Measurement, Wall, Door, LiftProperties, Location, Floor,
};
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};

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

struct Transition<T> {
    target: TargetTransition,
    placement: PlacementTransition<T>,
}

impl<T> From<(TargetTransition, PlacementTransition<T>)> for Transition<T> {
    fn from(input: (TargetTransition, PlacementTransition<T>)) -> Self {
        Self{
            target: input.0,
            placement: input.1,
        }
    }
}

impl<T: IntoSelectAnchorPlacement> From<Transition<T>> for Transition<SelectAnchorPlacement> {
    fn from(input: Transition<T>) -> Self {
        Transition{
            target: input.target,
            placement: input.placement.into()
        }
    }
}

struct TargetTransition {
    created: Option<Entity>,
    finished: bool,
}

impl TargetTransition {
    fn none() -> Self {
        Self{
            created: None,
            finished: false,
        }
    }

    fn create(e: Entity) -> Self {
        Self{
            created: Some(e),
            finished: false,
        }
    }

    fn finished() -> Self {
        Self{
            created: None,
            finished: true,
        }
    }

    fn finish(mut self) -> Self {
        self.finished = true;
        self
    }

    fn preview(&self, e: Option<Entity>) -> Option<Entity> {
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
        if self.finished {
            return None;
        }

        // The logic for next is the same as preview if the object is not
        // finished yet
        return self.preview(e);
    }
}

struct PlacementTransition<T> {
    preview: T,
    next: T,
}

impl<T: IntoSelectAnchorPlacement> From<PlacementTransition<T>> for PlacementTransition<SelectAnchorPlacement> {
    fn from(input: PlacementTransition<T>) -> Self {
        Self{
            preview: input.preview.into_sap(),
            next: input.next.into_sap(),
        }
    }
}

/// We make this custom trait instead of using the standard
/// `impl From<T> for SelectAnchorPlacement` to avoid a problematic trait
/// ambiguity that prevents compilation.
trait IntoSelectAnchorPlacement {
    fn into_sap(self) -> SelectAnchorPlacement;
}

impl IntoSelectAnchorPlacement for EdgePlacement<Lane<Entity>> {
    fn into_sap(self) -> SelectAnchorPlacement {
        SelectAnchorPlacement::Lane(self)
    }
}

impl IntoSelectAnchorPlacement for EdgePlacement<Measurement<Entity>> {
    fn into_sap(self) -> SelectAnchorPlacement {
        SelectAnchorPlacement::Measurement(self)
    }
}

impl IntoSelectAnchorPlacement for EdgePlacement<Wall<Entity>> {
    fn into_sap(self) -> SelectAnchorPlacement {
        SelectAnchorPlacement::Wall(self)
    }
}

impl IntoSelectAnchorPlacement for EdgePlacement<Door<Entity>> {
    fn into_sap(self) -> SelectAnchorPlacement {
        SelectAnchorPlacement::Door(self)
    }
}

impl IntoSelectAnchorPlacement for EdgePlacement<LiftProperties<Entity>> {
    fn into_sap(self) -> SelectAnchorPlacement {
        SelectAnchorPlacement::Lift(self)
    }
}

impl IntoSelectAnchorPlacement for PointPlacement<Location<Entity>> {
    fn into_sap(self) -> SelectAnchorPlacement {
        SelectAnchorPlacement::Location(self)
    }
}

impl IntoSelectAnchorPlacement for PathPlacement<Floor<Entity>> {
    fn into_sap(self) -> SelectAnchorPlacement {
        SelectAnchorPlacement::Floor(self)
    }
}

impl<T: IntoSelectAnchorPlacement> From<T> for SelectAnchorPlacement {
    fn from(input: T) -> Self {
        input.into_sap()
    }
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
    ) -> Result<Transition<Self>, ()> where Self: Sized;

    /// Check what anchor originally has this placement
    fn current<'w, 's>(
        &self,
        target: Entity,
        params: &SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> where Self: Sized;
}

#[derive(Debug, Clone, Copy)]
pub struct EdgePlacement<T> {
    side: Side,
    _ignore: std::marker::PhantomData<T>,
}

impl<T> PartialEq<EdgePlacement<T>> for EdgePlacement<T> {
    fn eq(&self, other: &EdgePlacement<T>) -> bool {
        self.side == other.side
    }
}

impl<T> Eq for EdgePlacement<T> { }

impl<T: Clone> EdgePlacement<T> {
    fn transition(&self) -> PlacementTransition<Self> {
        PlacementTransition{
            preview: self.clone(),
            next: Self{side: self.side.opposite(), _ignore: Default::default()},
        }
    }
}

impl<T: Bundle + From<Edge<Entity>> + Clone + std::fmt::Debug> Placement for EdgePlacement<T> {
    fn next<'w, 's>(
        &self,
        anchor_selection: Entity,
        replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition<Self>, ()> {
        let (target, mut endpoints) = match target {
            Some(target) => {
                // We expect that we already have an element and we are
                // modifying it.
                match params.edges.get_mut(target) {
                    Ok(edge) => {
                        (target, edge)
                    },
                    Err(_) => {
                        println!(
                            "DEV ERROR: Entity {:?} is not the right kind of \
                            element for {self:?}", target,
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
                let mut deps = match params.dependents.get_many_mut(anchors.array()) {
                    Ok(deps) => deps,
                    Err(_) => {
                        // One of the anchors was not a valid anchor, so we
                        // should abort.
                        println!(
                            "DEV ERROR: Invalid anchors being selected for \
                            {self:?}: {:?} and {:?}",
                            anchors.left(), anchors.right(),
                        );
                        return Err(());
                    }
                };

                let new_bundle: T = anchors.into();
                let target = params.commands
                    .spawn()
                    .insert_bundle(new_bundle)
                    .insert(Pending)
                    .id();
                for dep in &mut deps {
                    dep.dependents.insert(target);
                }

                return Ok((TargetTransition::create(target), self.transition()).into());
            }
        };

        // We are replacing one of the endpoints in an existing target
        let changed_anchors = match self.side {
            Side::Left => {
                match replacing {
                    Some(replacing) => {
                        if endpoints.right() == replacing {
                            // The right anchor was assigned the anchor that
                            // is being replaced, which means a flip happened
                            // previously, and the right anchor's original
                            // value is currently held by the left. We should
                            // give the right anchor back its original value
                            // which should be currently held by the left.
                            [Some(anchor_selection), Some(endpoints.left())]
                        } else if anchor_selection == endpoints.right() {
                            // The right anchor has been selected for the
                            // left, so flip the anchors.
                            [Some(anchor_selection), Some(replacing)]
                        } else {
                            // No need to modify the right anchor, just set
                            // the left to its new value.
                            [Some(anchor_selection), None]
                        }
                    },
                    None => {
                        println!(
                            "DEV ERROR: We should not be selecting a Left \
                            anchor if we are not in ReplaceAnchor mode."
                        );
                        return Err(());
                    }
                }
            },
            Side::Right => {
                match replacing {
                    Some(replacing) => {
                        if endpoints.left() == replacing {
                            // The left anchor was assigned the anchor that is
                            // being replaced, which means a flip happened
                            // previously, and the left anchor's original value
                            // is currently held by the left. We should give
                            // the left anchor back its original value which
                            // should be currently held by the right.
                            [Some(endpoints.right()), Some(anchor_selection)]
                        } else if anchor_selection == endpoints.left() {
                            // The left anchor has been selected for the right,
                            // so flip the anchors.
                            [Some(replacing), Some(anchor_selection)]
                        } else {
                            // No need to modify the left anchor, just set the
                            // right to its new value.
                            [None, Some(anchor_selection)]
                        }
                    },
                    None => {
                        [None, Some(anchor_selection)]
                    }
                }
            }
        };

        // Remove the target edge as a dependency from any anchors that are no
        // longer being used by this edge.
        for (changed, current) in &[
            (changed_anchors[0].is_some(), endpoints.left()),
            (changed_anchors[1].is_some(), endpoints.right()),
        ] {
            if *changed && changed_anchors.iter().find(|x| **x == Some(*current)).is_none() {
                // This anchor is being changed and is no longer being used by
                // the lane.
                if let Ok(mut deps) = params.dependents.get_mut(*current) {
                    deps.dependents.remove(&target);
                } else {
                    println!(
                        "DEV ERROR: No AnchorDependents component found for \
                        {:?} while in SelectAnchor mode.", *current
                    );
                }
            }
        }

        // Add the target edge as a dependency to any anchors that did not
        // previously have it.
        for changed in changed_anchors.iter().filter_map(|a| *a) {
            if endpoints.array().iter().find(|x| **x == changed).is_none() {
                if let Ok(deps) = params.dependents.get(changed) {
                    deps.dependents.contains(&target);
                } else {
                    println!(
                        "DEV ERROR: No AnchorDependents component found for \
                        {:?} while in SelectAnchor mode.",
                        changed
                    );
                }
            }
        }

        if let Some(a) = changed_anchors[0] {
            *endpoints.left_mut() = a;
        }

        if let Some(a) = changed_anchors[1] {
            *endpoints.right_mut() = a;
        }

        return match self.side {
            Side::Left => Ok((TargetTransition::none(), self.transition()).into()),
            Side::Right => Ok((TargetTransition::finished(), self.transition()).into()),
        };
    }

    fn current<'w, 's>(
        &self,
        target: Entity,
        params: &SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> {
        params.edges.get(target).ok().map(|edge| edge.side(self.side))
    }
}

impl<T> From<Side> for EdgePlacement<T> {
    fn from(side: Side) -> Self {
        Self{side, _ignore: Default::default()}
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PointPlacement<T> {
    _ignore: std::marker::PhantomData<T>,
}

impl<T> Default for PointPlacement<T> {
    fn default() -> Self {
        PointPlacement{_ignore: Default::default()}
    }
}

impl<T> PointPlacement<T> {
    fn transition(&self) -> PlacementTransition<Self> {
        PlacementTransition{
            preview: Default::default(),
            next: Default::default(),
        }
    }
}

impl<T: Bundle + From<Point<Entity>>> Placement for PointPlacement<T> {
    fn next<'w, 's>(
        &self,
        anchor_selection: Entity,
        _replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition<Self>, ()> {
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

                return Ok((TargetTransition::finished(), self.transition()).into());
            }
            None => {
                // The element doesn't exist yet, so we need to spawn one.
                let new_bundle: T = Point(anchor_selection).into();
                let target = params.commands
                    .spawn()
                    .insert_bundle(new_bundle)
                    .insert(Pending)
                    .id();
                if let Ok(mut dep) = params.dependents.get_mut(anchor_selection) {
                    dep.dependents.insert(target);
                } else {
                    println!("DEV ERROR: Unable to get anchor dependents for {anchor_selection:?}");
                }

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
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct PathPlacement<T> {
    /// Replace the floor anchor at the specified index, or push the anchor to
    /// the end if None is specified. If the specified index is too high, this
    /// value will be changed to None and all new anchors will be pushed to the
    /// back.
    placement: Option<usize>,
    _ignore: std::marker::PhantomData<T>,
}

impl<T> PathPlacement<T> {
    fn at_index(index: usize) -> Self {
        Self{placement: Some(index), _ignore: Default::default()}
    }

    fn start() -> Self {
        // Using None for the placement means anchors will always be inserted
        // at the end.
        Self{placement: None, _ignore: Default::default()}
    }

    fn transition_from(index: usize) -> PlacementTransition<Self> {
        PlacementTransition{
            preview: Self::at_index(index),
            next: Self::at_index(index+1),
        }
    }

    fn transition_to(index: usize) -> PlacementTransition<Self> {
        let index = if index > 0 { index - 1 } else { 0 };
        Self::transition_from(index)
    }
}

impl<T> From<Option<usize>> for PathPlacement<T> {
    fn from(p: Option<usize>) -> Self {
        PathPlacement{placement: p, _ignore: Default::default()}
    }
}

impl<T: Bundle + From<Path<Entity>> + std::fmt::Debug> Placement for PathPlacement<T> {
    fn next<'w, 's>(
        &self,
        anchor_selection: Entity,
        _replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition<Self>, ()> {
        let target = match target {
            Some(target) => target,
            None => {
                // We need to create a new element
                let new_bundle: T = Path(vec![anchor_selection]).into();
                let target = params.commands
                    .spawn()
                    .insert_bundle(new_bundle)
                    .insert(Pending)
                    .id();

                match params.dependents.get_mut(anchor_selection) {
                    Ok(mut dep) => {
                        dep.dependents.insert(target);
                    },
                    Err(_) => {
                        println!(
                            "DEV ERROR: Invalid anchor being selected for \
                            {self:?}",
                        );
                    }
                }

                return Ok((TargetTransition::create(target), Self::transition_from(0)).into());
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
                    return Ok((TargetTransition::finished(), Self::transition_from(index)).into());
                }
            }
        }

        if !behavior.allow_inner_loops {
            for (i, anchor) in path.iter().enumerate() {
                if *anchor == anchor_selection && i != index {
                    // The user has reselected a midpoint. That violates the
                    // requested behavior, so we ignore it.
                    return Ok((TargetTransition::none(), Self::transition_to(index)).into());
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
                    "DEV ERROR: Invalid anchor being selected for {self:?}",
                );
                return Ok((TargetTransition::none(), Self::transition_to(index)).into());
            }
        };
        dep.dependents.insert(target);

        return Ok((TargetTransition::none(), Self::transition_from(index)).into());
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

        let (path, _) = match params.paths.get(target) {
            Ok(p) => p,
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
}

#[derive(SystemParam)]
pub struct SelectAnchorPlacementParams<'w, 's> {
    edges: Query<'w, 's, &'static mut Edge<Entity>>,
    points: Query<'w, 's, &'static mut Point<Entity>>,
    paths: Query<'w, 's, (&'static mut Path<Entity>, &'static PathBehavior)>,
    dependents: Query<'w, 's, &'static mut AnchorDependents>,
    commands: Commands<'w, 's>,
    cursor: Res<'w, Cursor>,
}

#[derive(Debug, Clone, PartialEq)]
pub enum SelectAnchorPlacement {
    Lane(EdgePlacement<Lane<Entity>>),
    Measurement(EdgePlacement<Measurement<Entity>>),
    Wall(EdgePlacement<Wall<Entity>>),
    Door(EdgePlacement<Door<Entity>>),
    Lift(EdgePlacement<LiftProperties<Entity>>),
    Location(PointPlacement<Location<Entity>>),
    Floor(PathPlacement<Floor<Entity>>),
}

impl Placement for SelectAnchorPlacement {
    fn next<'w, 's>(
        &self,
        anchor_selection: Entity,
        replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut SelectAnchorPlacementParams<'w, 's>,
    ) -> Result<Transition<Self>, ()> {
        match self {
            Self::Lane(p) => p.next(
                anchor_selection, replacing, target, params,
            ).map(|x| x.into()),
            Self::Measurement(p) => p.next(
                anchor_selection, replacing, target, params,
            ).map(|x| x.into()),
            Self::Wall(p) => p.next(
                anchor_selection, replacing, target, params,
            ).map(|x| x.into()),
            Self::Door(p) => p.next(
                anchor_selection, replacing, target, params,
            ).map(|x| x.into()),
            Self::Lift(p) => p.next(
                anchor_selection, replacing, target, params,
            ).map(|x| x.into()),
            Self::Location(p) => p.next(
                anchor_selection, replacing, target, params,
            ).map(|x| x.into()),
            Self::Floor(p) => p.next(
                anchor_selection, replacing, target, params,
            ).map(|x| x.into()),
        }
    }

    fn current<'w, 's>(
        &self,
        target: Entity,
        params: &SelectAnchorPlacementParams<'w, 's>,
    ) -> Option<Entity> {
        match self {
            Self::Lane(p) => p.current(target, params),
            Self::Measurement(p) => p.current(target, params),
            Self::Wall(p) => p.current(target, params),
            Self::Door(p) => p.current(target, params),
            Self::Lift(p) => p.current(target, params),
            Self::Location(p) => p.current(target, params),
            Self::Floor(p) => p.current(target, params),
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
        SelectAnchor{
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Lift(self.placement.into()),
            continuity: self.continuity,
        }
    }

    pub fn for_measurement(self) -> SelectAnchor {
        SelectAnchor{
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Measurement(self.placement.into()),
            continuity: self.continuity,
        }
    }

    pub fn for_wall(self) -> SelectAnchor {
        SelectAnchor{
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Wall(self.placement.into()),
            continuity: self.continuity,
        }
    }

    pub fn for_door(self) -> SelectAnchor {
        SelectAnchor{
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Door(self.placement.into()),
            continuity: self.continuity,
        }
    }

    pub fn for_lift(self) -> SelectAnchor {
        SelectAnchor{
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Lift(self.placement.into()),
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
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Location(PointPlacement::default()),
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
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Floor(self.placement.into()),
            continuity: self.continuity,
        }
    }
}

/// This enum requests that the next selection should be an anchor, and that
/// selection should be provided to one of the enumerated entities. When the
/// inner object is None, that means the selection action should create a new
/// instance of one.
#[derive(Debug, Clone, PartialEq)]
pub struct SelectAnchor {
    for_element: Option<Entity>,
    placement: SelectAnchorPlacement,
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
            | SelectAnchorContinuity::Continuous => self.for_element.is_none(),
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
            self.for_element,
            params,
        ) {
            Ok(t) => t,
            Err(_) => { return None; }
        };

        if transition.target.finished {
            if let Some(finished_target) = transition.target.preview(self.for_element) {
                // Remove the Pending marker from the target because it has
                // been finished.
                params.commands.entity(finished_target).remove::<Pending>();
            } else {
                println!(
                    "DEV ERROR: An element was supposed to be finished by \
                    SelectAnchor, but we could not find it"
                );
            }
        }

        let next_target = transition.target.next(self.for_element);
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
                            for_element: Some(next_target),
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
                    for_element: next_target,
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
    ) -> Option<Self> {
        let transition = match self.placement.next(
            anchor_selection,
            self.continuity.replacing(),
            self.for_element,
            params,
        ) {
            Ok(t) => t,
            Err(_) => { return None; }
        };

        let target = match transition.target.preview(self.for_element) {
            Some(target) => target,
            None => {
                // This shouldn't happen. If a target wasn't already assigned
                // then a new one should have been created during the preview.
                // We'll just indicate that we should exit the current mode by
                // returning None.
                return None;
            }
        };

        Some(Self{
            for_element: Some(target),
            placement: transition.placement.preview,
            continuity: self.continuity
        })
    }
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

        if request.continuity.needs_original() {
            // Keep track of the original anchor that we intend to replace so
            // that we can revert any previews.
            let for_element = match request.for_element {
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

            let original = match request.placement.current(for_element, &params) {
                Some(original) => original,
                None => {
                    println!(
                        "DEV ERROR: cannot locate an original anchor for \
                        {:?} in entity {:?}. Reverting to Inspect Mode.",
                        request.placement,
                        for_element,
                    );
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };

            request.continuity = SelectAnchorContinuity::ReplaceAnchor{
                original_anchor: Some(original)
            };
        }
    }

    if hovering.is_changed() {
        dbg!();
        if hovering.0.is_none() {
            dbg!();
            set_visibility(params.cursor.frame, &mut visibility, true);
        } else {
            dbg!();
            set_visibility(params.cursor.frame, &mut visibility, false);
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
        } else {
            if let Some(target) = request.for_element {
                // Offer a preview based on the current hovering status
                let hovered = hovering.0.unwrap_or(params.cursor.anchor_placement);
                let current = request.placement.current(target, &params);

                if Some(hovered) != current {
                    // We should only call this function if the current hovered
                    // anchor is not the one currently assigned. Otherwise we
                    // are wasting query+command effort.
                    request = match request.preview(hovered, &mut params) {
                        Some(next_mode) => next_mode,
                        None => {
                            *mode = InteractionMode::Inspect;
                            return;
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
    }

    // TODO(MXG) Is there a nicer way of doing this?
    let need_change = if let InteractionMode::SelectAnchor(current) = &*mode {
        if *current == request {
            false
        } else {
            true
        }
    } else {
        true
    };

    if need_change {
        *mode = InteractionMode::SelectAnchor(request);
    }
}

