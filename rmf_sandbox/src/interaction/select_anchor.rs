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
    site::{AnchorBundle, AnchorDependents, Pending},
};
use rmf_site_format::{
    Side, Edge, Lane, Measurement, Wall, Door, Lift, Location, Floor,
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
            Self::ReplaceAnchor{original_anchor} => original_anchor,
            _ => None,
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
                        "DEV ERROR: Created a superfluous target while in "
                        "SelectAnchor mode"
                    );
                }
                Some(e)
            },
            None => {
                match self.created {
                    Some(e) => Some(e),
                    None => {
                        println!(
                            "DEV ERROR: Failed to create an entity while in "
                            "SelectAnchor mode"
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

impl<T, U: Into<T>> From<PlacementTransition<U>> for PlacementTransition<T> {
    fn from(input: PlacementTransition<U>) -> Self {
        Self{
            preview: input.preview.into(),
            next: input.next.into(),
        }
    }
}

trait Placement {
    type Params: SystemParam;

    /// Get what the next placement should be if an anchor is selected for the
    /// current placement. If None is returned, that means the element has been
    /// filled.
    fn next(
        &self,
        anchor_selection: Entity,
        replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut Self::Params,
        common: &mut SelectAnchorCommonParams,
    ) -> Result<(TargetTransition, PlacementTransition<Self>), ()>;

    /// Check what anchor originally has this placement
    fn current(
        &self,
        target: Entity,
        params: &Self::Params,
    ) -> Option<Entity>;
}

type ParamsOf<T> = <T as Placement>::Params;

#[derive(SystemParam)]
pub(crate) struct EdgePlacementParams<T> {
    edges: Query<&mut T>,
}

#[derive(Debug, Clone, Copy)]
pub(crate) struct EdgePlacement<T> {
    side: Side,
    _ignore: std::marker::PhantomData<T>,
}

impl<T> EdgePlacement<T> {
    fn transition(&self) -> PlacementTransition<Self> {
        PlacementTransition{
            preview: self.clone(),
            next: Self{side: self.side.opposite(), _ignore: Default::default()},
        }
    }
}

impl<T: Edge<Entity> + Component> Placement for EdgePlacement<T> {
    type Params = EdgePlacementParams<T>;
    fn next(
        &self,
        anchor_selection: Entity,
        replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut EdgePlacementParams<T>,
        common: &mut SelectAnchorCommonParams,
    ) -> (TargetTransition, PlacementTransition<Self>) {
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
                            "DEV ERROR: Entity {:?} is not the right kind of "
                            "element for {self:?}", target,
                        );
                        return Err(());
                    }
                }
            },
            None => {
                // We need to begin creating a new element
                let anchors = (anchor_selection, common.cursor.anchor_placement);
                let mut deps = match common.dependents.get_many_mut(
                    [anchors.0, anchors.1]
                ) {
                    Ok(deps) => deps,
                    Err(_) => {
                        // One of the anchors was not a valid anchor, so we
                        // should abort.
                        println!(
                            "DEV ERROR: Invalid anchors being selected for "
                            "{self:?}: {:?} and {:?}",
                            anchors.0, anchors.1
                        );
                        return Err(());
                    }
                };

                let target = common.commands
                    .spawn()
                    .insert(<T as Edge<Entity>>::new(anchors))
                    .insert(Pending)
                    .id();
                for dep in &mut deps {
                    dep.dependents.insert(target);
                }

                return Ok((TargetTransition::create(target), self.transition()));
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
                            "DEV ERROR: We should not be selecting a Left "
                            "anchor if we are not in ReplaceAnchor mode."
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
            if *changed && changed_anchors.iter().find(Some(*current)).is_none() {
                // This anchor is being changed and is no longer being used by
                // the lane.
                if let Ok(deps) = common.dependents.get_mut(*current) {
                    deps.dependents.remove(&target);
                } else {
                    println!(
                        "DEV ERROR: No AnchorDependents component found for "
                        "{:?} while in SelectAnchor mode.", *current
                    );
                }
            }
        }

        // Add the target edge as a dependency to any anchors that did not
        // previously have it.
        for changed in changed_anchors.iter().filter_map(|a| *a) {
            if endpoints.array().iter().find(changed).is_none() {
                if let Ok(deps) = common.dependents.get(changed) {
                    deps.dependents.contains(&target);
                } else {
                    println!(
                        "DEV ERROR: No AnchorDependents component found for "
                        "{:?} while in SelectAnchor mode.", *current
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

        match self.side {
            Side::Left => (TargetTransition::none(), self.transition()),
            Side::Right => (TargetTransition::finished(), self.transition()),
        }
    }

    fn current(
        &self,
        target: Entity,
        params: &Self::Params,
    ) -> Option<Entity> {
        params.edges.get(target).ok().map(|edge| edge.side(self.side))
    }
}

impl<T> From<Side> for EdgePlacement<T> {
    fn from(side: Side) -> Self {
        Self{side, ..default()}
    }
}

#[derive(Debug, SystemParam)]
pub(crate) struct LocationPlacementParams {
    locations: Query<&mut Location<Entity>>,
}

#[derive(Default, Debug, Clone, Copy)]
pub(crate) struct LocationPlacement;

impl LocationPlacement {
    fn transition(&self) -> PlacementTransition<Self> {
        PlacementTransition{
            preview: Default::default(),
            next: Default::default(),
        }
    }
}

impl Placement for LocationPlacement {
    type Params = LocationPlacementParams;
    fn next(
        &self,
        anchor_selection: Entity,
        replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut Self::Params,
        common: &mut SelectAnchorCommonParams,
    ) -> (Option<Entity>, Self) {
        match target {
            Some(target) => {
                // Change the anchor that the location is attached to.
                let location = match params.locations.get_mut(target) {
                    Ok(l) => l,
                    Err(_) => {
                        println!(
                            "DEV ERROR: Unable to get location {:?} while in "
                            "SelectAnchor mode.", target
                        );
                        return Err(());
                    }
                };

                if location.anchor != anchor_selection {
                    match common.dependents.get_many_mut(
                        [location.anchor, anchor_selection]
                    ) {
                        Ok([old_dep, new_dep]) => {
                            old_dep.dependents.remove(&target);
                            new_dep.dependents.insert(&target);
                        },
                        Err(_) => {
                            println!(
                                "DEV ERROR: Unable to get anchor dependents "
                                "for [{:?}, {:?}] while in SelectAnchor mode.",
                                location.anchor,
                                anchor_selection,
                            );
                            return Err(());
                        }
                    }
                }

                return (TargetTransition::finished(), self.transition());
            }
            None => {
                // The location doesn't exist yet, so we need to spawn one.
                let target = common.commands
                    .spawn()
                    .insert(Location{
                        anchor: anchor_selection,
                        tags: Default::default(),
                    })
                    .insert(Pending)
                    .id();
                if let Ok(dep) = common.dependents.get_mut(anchor_selection) {
                    dep.dependents.insert(target);
                } else {
                    println!("DEV ERROR: Unable to get anchor dependents for {anchor_selection:?}");
                }

                return (TargetTransition::create(target).finish(), self.transition());
            }
        }
    }

    fn current(
        &self,
        target: Entity,
        params: &Self::Params,
    ) -> Option<Entity> {
        params.locations.get(target).ok().map(|location| location.anchor)
    }
}

#[derive(Debug, SystemParam)]
pub(crate) struct FloorPlacementParams {
    floors: Query<&mut Floor<Entity>>,
}

#[derive(Debug)]
pub(crate) struct FloorPlacement {
    /// Replace the floor anchor at the specified index, or push the anchor to
    /// the end if None is specified. If the specified index is too high, this
    /// value will be changed to None and all new anchors will be pushed to the
    /// back.
    placement: Option<usize>,
}

impl FloorPlacement {
    fn at_index(index: usize) -> Self {
        Self{placement: Some(index)}
    }

    fn start() -> Self {
        // Using None for the placement means anchors will always be inserted
        // at the end.
        Self{placement: None}
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

impl Placement for FloorPlacement {
    type Params = FloorPlacementParams;

    fn next(
        &self,
        anchor_selection: Entity,
        replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut Self::Params,
        common: &mut SelectAnchorCommonParams,
    ) -> Result<(TargetTransition, Self), ()> {
        let target = match target {
            Some(target) => target,
            None => {
                // We need to create a new floor
                let target = common.commands
                    .spawn()
                    .insert(Floor{
                        anchors: vec![anchor_selection],
                        texture: None,
                    })
                    .insert(Pending)
                    .id();

                match common.dependents.get_mut(anchor_selection) {
                    Ok(dep) => {
                        dep.dependents.insert(target);
                    },
                    Err(_) => {
                        println!(
                            "DEV ERROR: Invalid anchor being selected for "
                            "{self:?}", self,
                        );
                    }
                }

                return Ok((TargetTransition::create(target), Self::transition_from(0)));
            }
        };

        let floor = match params.floors.get_mut(target) {
            Ok(floor) => floor,
            Err(_) => {
                println!(
                    "DEV ERROR: Unable to find floor {target:?} while in "
                    "SelectAnchor mode."
                );
                return Err(());
            }
        };

        let index = self.placement.unwrap_or(floor.anchors.len()).min(floor.anchors.len());
        if floor.anchors.len() >= 3 {
            if Some(anchor_selection) == floor.anchors.first() {
                if index >= floor.anchors.len() - 1 {
                    // The user has set the first node to the last node,
                    // creating a closed loop. We should consider the floor to
                    // be finished.
                    if index == floor.anchors.len() - 1 {
                        // Remove the last element because it is redundant with
                        // the first element now.
                        floor.anchors.pop();
                    }
                    return Ok((TargetTransition::finished(), Self::transition_from(index)));
                }
            }
        }

        for (i, anchor) in floor.anchors.iter().enumerate() {
            if *anchor == anchor_selection && i != index {
                // The user has reselected a midpoint. That doesn't make sense
                // for a floor so we will disregard it.
                return Ok((TargetTransition::none(), Self::transition_to(index)));
            }
        }

        if let Some(place_anchor) = floor.anchors.get_mut(index) {
            let old_anchor = *place_anchor;
            *place_anchor = anchor_selection;

            if floor.anchors.iter().find(old_anchor).is_none() {
                if let Ok(mut dep) = common.dependents.get_mut(old_anchor) {
                    // Remove the dependency for the old anchor since we are not
                    // using it anymore.
                    dep.dependents.remove(&target);
                } else {
                    println!(
                        "DEV ERROR: Invalid old anchor {:?} in floor", old_anchor
                    );
                }
            } else {
                println!(
                    "DEV ERROR: Anchor {old_anchor:?} was duplicated in a floor"
                );
            }
        } else {
            // We need to add this anchor to the end of the vector
            floor.anchors.push(anchor_selection);
        }

        let mut dep = match common.dependents.get_mut(anchor_selection) {
            Ok(dep) => dep,
            Err(_) => {
                println!(
                    "DEV ERROR: Invalid anchor being selected for {self:?}",
                    self
                );
                return Ok((TargetTransition::none(), Self::transition_to(index)));
            }
        };
        dep.dependents.insert(target);

        return Ok((TargetTransition::none(), Self::transition_from(index)));
    }

    fn current(
        &self,
        target: Entity,
        params: &Self::Params,
    ) -> Option<Entity> {
        let index = match self.placement {
            Some(i) => i,
            None => { return None; }
        };

        let floor = match params.floors.get(target) {
            Ok(f) => f,
            Err(_) => {
                println!(
                    "DEV ERROR: Unable to find floor {:?} while in "
                    "SelectAnchor mode", target,
                );
                return None;
            }
        };

        floor.anchors.get(i)
    }
}

#[derive(SystemParam, Debug)]
struct SelectAnchorPlacementParams {
    lane: ParamsOf<EdgePlacement<Lane<Entity>>>,
    measurement: ParamsOf<EdgePlacement<Measurement<Entity>>>,
    wall: ParamsOf<EdgePlacement<Wall<Entity>>>,
    door: ParamsOf<EdgePlacement<Door<Entity>>>,
    lift: ParamsOf<EdgePlacement<Lift<Entity>>>,
    location: ParamsOf<LocationPlacement>,
    floor: ParamsOf<FloorPlacement>,
}

#[derive(SystemParam, Debug)]
struct SelectAnchorCommonParams {
    commands: Commands,
    cursor: Res<Cursor>,
    dependents: Query<&mut AnchorDependents>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectAnchorPlacement {
    Lane(EdgePlacement<Lane<Entity>>),
    Measurement(EdgePlacement<Measurement<Entity>>),
    Wall(EdgePlacement<Wall<Entity>>),
    Door(EdgePlacement<Door<Entity>>),
    Lift(EdgePlacement<Lift<Entity>>),
    Location(LocationPlacement),
    Floor(FloorPlacement),
}

impl Placement for SelectAnchorPlacement {
    type Params = SelectAnchorPlacementParams;

    fn next(
        &self,
        anchor_selection: Entity,
        replacing: Option<Entity>,
        target: Option<Entity>,
        params: &mut Self::Params,
        common: &mut SelectAnchorCommonParams,
    ) -> Result<(TargetTransition, PlacementTransition<Self>), ()> {
        match self {
            Self::Lane(p) => p.next(
                anchor_selection, replacing, target, &mut params.lane, common
            ).map(|x| x.into()),
            Self::Measurement(p) => p.next(
                anchor_selection, replacing, target, &mut params.measurement, common,
            ).map(|x| x.into()),
            Self::Wall(p) => p.next(
                anchor_selection, replacing, target, &mut params.wall, common,
            ).map(|x| x.into()),
            Self::Door(p) => p.next(
                anchor_selection, replacing, target, &mut params.door, common,
            ).map(|x| x.into()),
            Self::Lift(p) => p.next(
                anchor_selection, replacing, target, &mut params.lift, common
            ).map(|x| x.into()),
            Self::Location(p) => p.next(
                anchor_selection, replacing, target, &mut params.location, common
            ).map(|x| x.into()),
            Self::Floor(p) => p.next(
                anchor_selection, replacing, target, &mut params.floor, common,
            ).map(|x| x.into()),
        }
    }

    fn preview(
        &self,
        anchor_candidate: Entity,
        target: Option<Entity>,
        params: &mut Self::Params,
        common: &mut SelectAnchorCommonParams,
    ) {
        match self {
            Self::Lane(p) => p.preview(
                anchor_candidate, target, &mut params.lane, common
            ),
            Self::Measurement(p) => p.preview(
                anchor_candidate, target, &mut params.measurement, common
            ),
            Self::Wall(p) => p.preview(
                anchor_candidate, target, &mut params.wall, common
            ),
            Self::Door(p) => p.preview(
                anchor_candidate, target, &mut params.door, common
            ),
            Self::Lift(p) => p.preview(
                anchor_candidate, target, &mut params.lift, common
            ),
            Self::Location(p) => p.preview(
                anchor_candidate, target, &mut params.location, common
            ),
            Self::Floor(p) => p.preview(
                anchor_candidate, target, &mut params.floor, common
            ),
        }
    }

    fn current(
        &self,
        target: Entity,
        params: &Self::Params,
    ) -> Option<Entity> {
        match self {
            Self::Lane(p) => p.current(target, &params.lane),
            Self::Measurement(p) => p.current(target, &params.measurement),
            Self::Wall(p) => p.current(target, &params.wall),
            Self::Door(p) => p.current(target, &params.door),
            Self::Lift(p) => p.current(target, &params.lift),
            Self::Location(p) => p.current(target, &params.location),
            Self::Floor(p) => p.current(target, &params.floor),
        }
    }
}

impl<T: Into<SelectAnchorPlacement>>
From<(TargetTransition, PlacementTransition<T>)>
for (TargetTransition, PlacementTransition<SelectAnchorPlacement>) {
    fn from(input: (TargetTransition, PlacementTransition<T>)) -> Self {
        (input.0, input.1.into())
    }
}

impl From<EdgePlacement<Lane<Entity>>> for SelectAnchorPlacement {
    fn from(input: EdgePlacement<Lane<Entity>>) -> Self {
        SelectAnchorPlacement::Lane(input)
    }
}

impl From<EdgePlacement<Measurement<Entity>>> for SelectAnchorPlacement {
    fn from(input: EdgePlacement<Measurement<Entity>>) -> Self {
        SelectAnchorPlacement::Measurement(input)
    }
}

impl From<EdgePlacement<Wall<Entity>>> for SelectAnchorPlacement {
    fn from(input: EdgePlacement<Wall<Entity>>) -> Self {
        SelectAnchorPlacement::Wall(input)
    }
}

impl From<EdgePlacement<Door<Entity>>> for SelectAnchorPlacement {
    fn from(input: EdgePlacement<Door<Entity>>) -> Self {
        SelectAnchorPlacement::Door(input)
    }
}

impl From<EdgePlacement<Lift<Entity>>> for SelectAnchorPlacement {
    fn from(input: EdgePlacement<Lift<Entity>>) -> Self {
        SelectAnchorPlacement::Lift(input)
    }
}

impl From<LocationPlacement> for SelectAnchorPlacement {
    fn from(input: LocationPlacement) -> Self {
        SelectAnchorPlacement::Location(input)
    }
}

impl From<FloorPlacement> for SelectAnchorPlacement {
    fn from(input: FloorPlacement) -> Self {
        SelectAnchorPlacement::Floor(input)
    }
}

pub struct SelectAnchorEdgeBuilder {
    for_element: Option<Entity>,
    placement: Side,
    continuity: SelectAnchorContinuity,
}

impl SelectAnchorEdgeBuilder {
    pub fn for_lane(self) -> SelectAnchor where P: Side {
        SelectAnchor{
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Lift(self.placement.into()),
            continuity: self.continuity,
        }
    }

    pub fn for_measurement(self) -> SelectAnchor where P: Side {
        SelectAnchor{
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Measurement(self.placement.into()),
            continuity: self.continuity,
        }
    }

    pub fn for_wall(self) -> SelectAnchor where P: Side {
        SelectAnchor{
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Wall(self.placement.into()),
            continuity: self.continuity,
        }
    }

    pub fn for_door(self) -> SelectAnchor where P: Side {
        SelectAnchor{
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Door(self.placement.into()),
            continuity: self.continuity,
        }
    }

    pub fn for_lift(self) -> SelectAnchor where P: Side {
        SelectAnchor{
            for_element: self.for_element,
            placement: SelectAnchorPlacement::Lift(self.placement.into()),
            continuity: self.continuity,
        }
    }
}

/// This enum requests that the next selection should be an anchor, and that
/// selection should be provided to one of the enumerated entities. When the
/// inner object is None, that means the selection action should create a new
/// instance of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
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
    pub fn create_new_path() -> SelectAnchorEdgeBuilder {
        SelectAnchorEdgeBuilder{
            for_element: None,
            placement: Side::Left,
            continuity: SelectAnchorContinuity::Continuous,
        }
    }

    /// Create one new location. After an anchor is selected the new location
    /// will be created and the mode will return to Inspect.
    pub fn create_new_location() -> SelectAnchor {
        SelectAnchor{
            for_element: None,
            placement: LocationPlacement,
            continuity: SelectAnchorContinuity::InsertElement,
        }
    }

    /// Move an existing location to a new anchor.
    pub fn move_location(
        location: Entity,
        original_anchor: Entity,
    ) -> SelectAnchor {
        SelectAnchor{
            for_element: Some(location),
            placement: LocationPlacement,
            continuity: SelectAnchorContinuity::ReplaceAnchor{
                original_anchor: None,
            }
        }
    }

    /// Create a new floor. The user will be able to select anchors continuously
    /// until they Backout. If the user selects an anchor that is already part
    /// of the floor the selection will be ignored, unless it is the first
    /// anchor of the floor, in which case a Backout will occur.
    pub fn create_new_floor() -> SelectAnchor {
        SelectAnchor{
            for_element: None,
            placement: FloorPlacement::start(),
            continuity: SelectAnchorContinuity::Continuous,
        }
    }

    /// Replace which anchor one of the points on the floor is using.
    pub fn replace_floor_point(
        floor: Entity,
        usize: index,
    ) -> SelectAnchor {
        SelectAnchor {
            for_element:  Some(floor),
            placement: FloorPlacement::at_index(usize),
            continuity: SelectAnchorContinuity::ReplaceAnchor{
                original_anchor: None,
            },
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
    fn next(
        &self,
        anchor_selection: Entity,
        params: &mut SelectAnchorPlacementParams,
        common: &mut SelectAnchorCommonParams,
    ) -> Option<Self> {
        let (target_transition, placement_transition) = match self.placement.next(
            anchor_selection,
            self.continuity.replacing(),
            self.for_element,
            params,
            common,
        ) {
            Ok(t) => t,
            Err(_) => { return None; }
        };

        let next_target = target_transition.next(self.for_element);
        let next_placement = placement_transition.next;

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

    fn preview(
        &self,
        anchor_selection: Entity,
        params: &mut SelectAnchorPlacementParams,
        common: &mut SelectAnchorCommonParams,
    ) -> Option<Self> {
        let (target_transition, placement_transition) = match self.placement.next(
            anchor_selection,
            self.continuity.replacing(),
            self.for_element,
            params,
            common,
        ) {
            Ok(t) => t,
            Err(_) => { return None; }
        };
        let target = match target_transition.preview(self.for_element) {
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
            placement: placement_transition.preview,
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
    mut placement_params: SelectAnchorPlacementParams,
    mut common_params: SelectAnchorCommonParams,
    mut visibility: Query<&mut Visibility>,
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
    mut select: EventReader<Select>,
    mut hover: EventWriter<Hover>,
) {
    let mut request = match *mode {
        InteractionMode::SelectAnchor(request) => request,
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
                        "DEV ERROR: for_element must be Some for ReplaceAnchor. "
                        "Reverting to Inspect Mode."
                    );
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };

            let original = match request.placement.current(for_element, &params) {
                Some(original) => original,
                None => {
                    println!(
                        "DEV ERROR: cannot locate an original anchor for "
                        "{:?} in entity {:?}. Reverting to Inspect Mode.",
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
        if hovering.0.is_none() {
            set_visibility(common_params.cursor.frame, &mut visibility, true);
        } else {
            set_visibility(common_params.cursor.frame, &mut visibility, false);
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
            let new_anchor = common_params.commands
                .spawn_bundle(
                    AnchorBundle::at_transform(common_params.cursor.frame)
                ).id();

            request = match request.next(
                new_anchor, &mut placement_params, &mut common_params,
            ) {
                Some(next_mode) => next_mode,
                None => {
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };
        } else {
            if let Some(target) = request.for_element {
                // Offer a preview based on the current hovering status
                let hovered = hovering.0.unwrap_or(
                    common_params.cursor.anchor_placement
                );

                let current = request.placement.current(
                    target, &placement_params
                );

                if Some(hovered) != current {
                    // We should only call this function if the current hovered
                    // anchor is not the one currently assigned. Otherwise we
                    // are wasting query+command effort.
                    request = match request.preview(
                        hovered, &mut placement_params, &mut common_params
                    ) {
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
            request = match request.next(
                new_selection, &mut placement_params, &mut common_params,
            ) {
                Some(next_mode) => next_mode,
                None => {
                    *mode = InteractionMode::Inspect;
                    return;
                }
            };
        }
    }

    if InteractionMode::SelectAnchor(request) != *mode {
        // Update the interaction mode to the most recent request mode
        *mode = InteractionMode::SelectAnchor(request);
    }
}

