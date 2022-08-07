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

use crate::*;
use std::collections::BTreeMap;
use serde::{Serialize, Deserialize};

const DEFAULT_CABIN_WALL_THICKNESS: f32 = 0.05;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lift<SiteID: Ord> {
    /// Name of this lift. This must be unique within the site.
    pub name: String,
    /// These anchors define the canonical reference frame of the lift. Both
    /// anchors must belong to the same level.
    pub anchors: (SiteID, SiteID),
    /// Description of the cabin for the lift.
    pub cabin: LiftCabin,
    /// A map from the ID of a level that this lift can visit to the door that
    /// the lift opens on that level. key: level, value: door. The lift can only
    /// visit levels that are included in this map.
    pub level_doors: BTreeMap<SiteID, SiteID>,
    /// For each level (key of the map, given as its ID in the [`Site`]::levels
    /// map), specify two anchors that correct the positioning of this lift on
    /// that level. These will act like [`Fiducial`] when the site levels are
    /// being aligned.
    ///
    /// Finalized site files should not have this field because it should become
    /// unnecessary after levels have been scaled and aligned.
    pub corrections: BTreeMap<u32, (SiteID, SiteID)>,
    /// When this is true, the lift is only for decoration and will not be
    /// responsive during a simulation.
    pub is_static: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum LiftCabin {
    /// The
    Params{
        /// How wide is the interior of the cabin, along the axis formed by the
        /// anchor points.
        width: f32,
        /// How deep is the cabin, i.e. interior distance from the front wall to
        /// the back wall of the cabin.
        depth: f32,
        /// What type of door is attached to the cabin.
        door: LiftCabinDoor,
        /// How thick are the walls of the cabin. Default is 0.05m.
        #[serde(skip_serializing_if="Option::is_none")]
        wall_thickness: Option<f32>,
        /// How large is the gap between the line formed by the anchor points
        /// and the edge of the cabin that lines up with the door. Default is
        /// 0.01m.
        #[serde(skip_serializing_if="Option::is_none")]
        gap: Option<f32>,
        /// Left (positive) / right (negative) shift of the cabin, off-center
        /// from the anchor points. Default is 0.0m.
        #[serde(skip_serializing_if="Option::is_none")]
        shift: Option<f32>,
    },
    /// The model pose is relative to the center point of the two Lift anchors,
    /// with the y-axis facing the left anchor. The lift doors should open along
    /// the +/- y-axis, and agents should exit the lift along the positive x-axis.
    Model(Model),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct LiftCabinDoor {
    /// How wide is the lift cabin door
    pub width: f32,
    /// What kind of door is this
    pub kind: DoorType,
    /// Shift the door off-center to the left (positive) or right (negative)
    pub shifted: Option<f32>,
}
