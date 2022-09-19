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
    ) -> (Option<Entity>, Self);

    /// Preview what it would look like for the candidate anchor to be selected
    fn preview(
        &self,
        anchor_candidate: Entity,
        target: Option<Entity>,
        params: &mut Self::Params,
        common: &mut SelectAnchorCommonParams,
    );

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
    fn left() -> Self {
        Self{side: Side::Left, ..default()}
    }

    fn right() -> Self {
        Self{side: Side::Right, ..default()}
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
    ) -> (Option<Entity>, Self) {
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
                            "element for {self:?}"
                        );
                        return (None, self::right());
                    }
                }
            },
            None => {
                // We need to begin creating a new element
                let anchors = (anchor_selection, common.cursor.anchor_placement);
                let deps = match common.dependents.get_many_mut(
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
                        return (None, self::right());
                    }
                };

                let target = common.commands
                    .spawn_bundle(<T as Edge<Entity>>::new(anchors))
                    .insert(Pending)
                    .id();
                for dep in &mut deps {
                    dep.dependents.insert(target);
                }

                return (Some(target), self::right());
            }
        };

        // We are replacing one of the endpoints in an existing target
        let changed_anchors = match self.side {
            Side::Left => {
                if replacing == Some(anchor_selection) {
                    // The user has selected the anchor that was originally
                    // meant to be replaced so we should accept that choice.
                    (Some(anchor_selection), None)
                } else {
                    // A new anchor is being selected
                    if anchor_selection == endpoints.right() {
                        // The anchor selected for the left is the same as the
                        // anchor already being used for the right.
                        match replacing {
                            Some(replacing) => {
                                if replacing == endpoints.right() {
                                    // Since the right side has the

                                } else {

                                }
                            }
                        }
                    }
                }
            },
            Side::Right => {

            }
        }
    }

    fn start() -> Self {
        Self::left()
    }
}

impl<T> From<Side> for EdgePlacement<T> {
    fn from(side: Side) -> Self {
        Self{side, ..default()}
    }
}

pub(crate) struct LocationPlacement;

impl Placement for LocationPlacement {
    fn next(&self) -> Option<Self> {
        None
    }

    fn start() -> Self {
        Self
    }
}

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
}

impl Placement for FloorPlacement {
    fn next(&self) -> Option<Self> {
        Some(Self{placement: self.placement.map(|i| i+1)})
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
    ) -> (Option<Entity>, Self) {
        match self {
            Self::Lane(p) => p.next(
                anchor_selection, replacing, target, &mut params.lane, common
            ).map(Self::Lane),
            Self::Measurement(p) => p.next(
                anchor_selection, replacing, target, &mut params.measurement, common,
            ).map(Self::Measurement),
            Self::Wall(p) => p.next(
                anchor_selection, replacing, target, &mut params.wall, common,
            ).map(Self::Wall),
            Self::Door(p) => p.next(
                anchor_selection, replacing, target, &mut params.door, common,
            ).map(Self::Door),
            Self::Lift(p) => p.next(
                anchor_selection, replacing, target, &mut params.lift, common
            ).map(Self::Lift),
            Self::Location(p) => p.next(
                anchor_selection, replacing, target, &mut params.location, common
            ).map(Self::Location),
            Self::Floor(p) => p.next(
                anchor_selection, replacing, target, &mut params.floor, common,
            ).map(Self::Floor)
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
        let (next_target, next_placement) = self.placement.next(
            anchor_selection,
            self.continuity.replacing(),
            self.for_element,
            params,
            common,
        );

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
                    request.next(
                        hovered, &mut placement_params, &mut common_params
                    );

                    // Note that we do not save the next mode because we are
                    // really just previewing things. We keep the mode as-is
                    // until a selection has occurred.
                    return;
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

