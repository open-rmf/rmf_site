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
    site::AnchorDependents,
};
use rmf_site_format::{Lane, Side, Edge, Location, Floor};
use bevy::prelude::*;

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

trait Placement<T> {
    type Params;

    /// Get what the next placement should be if an anchor is selected for the
    /// current placement. If None is returned, that means the element has been
    /// filled.
    fn next(
        &self,
        anchor_selection: Entity,
        target: Option<Entity>,
        params: &mut Self::Params,
        dependents: &mut Query<&mut AnchorDependents>,
    ) -> (Option<Entity>, Self);

    /// Preview what it would look like for the candidate anchor to be selected
    fn preview(
        &self,
        anchor_candidate: Entity,
        target: Option<Entity>,
        params: &mut Self::Params,
        dependents: &mut Query<&mut AnchorDependents>,
    );

    fn original(
        &self,
        target: Entity,
        params: &Self::Params,
    ) -> Option<Entity>;

    fn start() -> Self;
}

pub(crate) struct EdgeNextPlacementParams<T> {
    commands: Commands,
    edges: Query<&mut T>,
}

impl<T: Edge<Entity>> Placement<T> for Side {
    type NextParams = EdgeNextPlacementParams<T>;
    fn next(
        &self,
        anchor_selection: Entity,
        params: &mut EdgeNextPlacementParams<T>,
        dependents: &mut Query<&mut AnchorDependents>,
    ) -> Option<Side> {
        match self {
            Side::Left => {

                Some(Side::Right)
            },
            Side::Right => {

                None
            },
        }
    }

    fn start() -> Self {
        Side::Left
    }
}

impl Placement<Location> for () {
    fn next(&self) -> Option<Self> {
        None
    }

    fn start() -> Self {
        ()
    }
}

impl Placement<Floor> for Option<usize> {
    fn next(&self) -> Option<Self> {
        Some(self.map(|i| i+1))
    }

    fn start() -> Self {
        // Using None for the placement means anchors will always be inserted
        // at the end.
        None
    }
}

/// Describe the behavior of selecting an anchor for an edge-like element
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SelectAnchorBehavior<P=Side> {
    /// The target that the anchor should be given to. A None value means the
    /// anchor selection should begin the creation of a new element.
    pub target: Option<Entity>,
    pub placement: P,
    /// See [`SelectAnchorContinuity`]
    pub continuity: SelectAnchorContinuity,
}

impl<P: Placement> SelectAnchorBehavior<P> {

    pub fn into_lane(self) -> SelectAnchor where P: Side {
        SelectAnchor::ForLane(self)
    }

    pub fn into_measurement(self) -> SelectAnchor where P: Side {
        SelectAnchor::ForMeasurement(self)
    }

    pub fn into_wall(self) -> SelectAnchor where P: Side {
        SelectAnchor::ForWall(self)
    }

    pub fn into_door(self) -> SelectAnchor where P: Side {
        SelectAnchor::ForDoor(self)
    }

    pub fn into_lift(self) -> SelectAnchor where P: Side {
        SelectAnchor::ForLift(self)
    }

    /// Get what the next mode should be if an anchor is selected during the
    /// current mode. If None is returned, that means we are done selecting
    /// anchors and should return to Inspect mode.
    fn next(
        &self,
        anchor_selection: Entity,
        next_params: &P::NextParams,
        start_params: &P::StartParams,
    ) -> Option<Self> {
        match self.target {
            Some(target) => {
                match self.continuity {
                    SelectAnchorContinuity::ReplaceAnchor => {
                        return None;
                    },
                    SelectAnchorContinuity::InsertElement => {
                        return target.1.next(anchor_selection, next_params)
                            .map(|p| {
                                Self{
                                    target: Some((target.0, p)),
                                    continuity: self.continuity,
                                }
                            });
                    },
                    SelectAnchorContinuity::Continuous => {
                        return Some(Self{
                            target: target.1.next(anchor_selection, next_params)
                                .map(|p| {
                                    (target.0, p)
                                }),
                            continuity: self.continuity,
                        });
                    }
                }
            },
            None => {
                return Self{
                    target: (placeholder, P::start(start_params)),
                    continuity: self.continuity,
                }.next(anchor_selection, next_params, start_params);
            }
        }
    }

    fn original(
        &self,
        params: &P::Params,
    ) -> Option<Entity> {
        self.target.map(|target| {
            self.placement.original(target, params)
        }).flatten()
    }

    /// Get what the next mode should be if a backout happens. If None is
    /// returned, that means we are done selecting anchors and should return to
    /// Inspect mode.
    // fn backout(&self) -> Option<Self> {
    //     match self.continuity {
    //         SelectAnchorContinuity::ReplaceAnchor => {
    //             return None;
    //         },
    //         SelectAnchorContinuity::InsertElement => {

    //         }
    //     }
    // }
}

/// This enum requests that the next selection should be an anchor, and that
/// selection should be provided to one of the enumerated entities. When the
/// inner object is None, that means the selection action should create a new
/// instance of one.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SelectAnchor {
    ForLane(SelectAnchorBehavior<Side>),
    ForMeasurement(SelectAnchorBehavior),
    ForWall(SelectAnchorBehavior),
    ForDoor(SelectAnchorBehavior),
    ForLift(SelectAnchorBehavior),
    /// A location has exactly one anchor, so just knowing which entity is
    /// sufficient.
    ForLocation(SelectAnchorBehavior<()>),
    /// Replace the floor anchor at the specified index, or push the anchor to
    /// the end if None is specified. If the specified index is too high, the
    /// anchor will simply be pushed.
    ForFloor(SelectAnchorBehavior<Option<usize>>),
}

impl SelectAnchor {

    pub fn replace_side(
        edge: Entity,
        side: Side,
    ) -> SelectAnchorBehavior {
        SelectAnchorBehavior{
            target: Some(edge),
            placement: Side,
            continuity: SelectAnchorContinuity::ReplaceAnchor{
                original_anchor: None,
            }
        }
    }

    /// Create a single new element of some edge-like type, e.g. Lane, Wall.
    ///
    /// # Examples
    ///
    /// ```
    /// let mode = SelectAnchor::create_one_new_edge().into_lane();
    /// ```
    pub fn create_one_new_edge() -> SelectAnchorBehavior {
        SelectAnchorBehavior{
            target: None,
            placement: Side::start(),
            continuity: SelectAnchorContinuity::InsertElement
        }
    }

    /// Creates a new path of elements for some edge-like type, e.g. Lane, Wall.
    /// New elements will be continuously produced until the user backs out.
    ///
    /// # Examples
    ///
    /// ```
    /// let mode = SelectAnchor::create_new_path().into_wall();
    /// ```
    pub fn create_new_path() -> SelectAnchorBehavior {
        SelectAnchorBehavior{
            target: None,
            placement: Side::start(),
            continuity: SelectAnchorContinuity::Continuous
        }
    }

    /// Create one new location. After an anchor is selected the new location
    /// will be created and the mode will return to Inspect.
    pub fn create_new_location() -> SelectAnchor {
        Self::ForLocation(
            SelectAnchorBehavior{
                target: None,
                placement: (),
                continuity: SelectAnchorContinuity::InsertElement,
            }
        )
    }

    /// Move an existing location to a new anchor.
    pub fn move_location(
        location: Entity,
        original_anchor: Entity,
    ) -> SelectAnchor {
        Self::ForLocation(
            SelectAnchorBehavior{
                target: Some(location),
                placement: (),
                continuity: SelectAnchorContinuity::ReplaceAnchor{
                    original_anchor: None,
                }
            }
        )
    }

    /// Create a new floor. The user will be able to select anchors continuously
    /// until they Backout. If the user selects an anchor that is already part
    /// of the floor the selection will be ignored, unless it is the first
    /// anchor of the floor, in which case a Backout will occur.
    pub fn create_new_floor() -> SelectAnchor {
        Self::ForFloor(
            SelectAnchorBehavior{
                target: None,
                placement: None,
                continuity: SelectAnchorContinuity::Continuous,
            }
        )
    }

    /// Replace which anchor one of the points on the floor is using.
    pub fn replace_floor_point(
        floor: Entity,
        usize: index,
    ) -> SelectAnchor {
        SelectAnchor::ForFloor(
            SelectAnchorBehavior{
                target: Some(floor),
                placement: Some(usize),
                continuity: SelectAnchorContinuity::ReplaceAnchor{
                    original_anchor: None,
                }
            }
        )
    }

    /// Whether a new object is being created
    pub fn creating(&self) -> bool {
        match self {
            Self::ForLane(b) | Self::ForMeasurement(b)
            | Self::ForWall(b) | Self::ForDoor(b) | Self::ForLift(b)
            | Self::ForLocation(b) | Self::ForFloor(b) => b.target.is_none(),
            _ => false,
        }
    }
}

fn process_behavior<T: Edge<Entity> + Component>(
    mode: &InteractionMode,
    new_selection: Entity,
    query: &mut Query<&mut T>,
) -> Option<SelectAnchorBehavior> {

}

pub fn handle_select_anchor_mode(
    mut mode: ResMut<InteractionMode>,
    anchors: Query<(), With<Anchor>>,
    hovering: Res<Hovering>,
    cursor: Res<Cursor>,
    mut lanes: Query<&mut Lane<Entity>>,
    mut dependents: Query<&mut AnchorDependents>,
    mut visibility: Query<&mut Visibility>,
    mut selected: Query<&mut Selected>,
    mut selection: ResMut<Selection>,
    mut select: EventReader<Select>,
    mut hover: EventWriter<Hover>,
) {
    let request = match *mode {
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
        if request.creating() {
            if let Some(previous_selection) = selection.0 {
                if let Ok(mut selected) = selected.get_mut(previous_selection) {
                    selected.is_selected = false;
                }
                selection.0 = None;
            }
        }


    }

    if hovering.is_changed() {
        if hovering.0.is_none() {
            set_visibility(cursor.frame, &mut visibility, true);
        }
    }

    for new_selection in select.iter().filter_map(|s| s.0) {
        let next_mode = match &request {
            SelectAnchor::ForLane(b) => {

            }
            _ => None,
        };
    }
}

