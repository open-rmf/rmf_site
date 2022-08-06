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

pub enum LiftCabin {
    /// The
    Params{
        /// How large is the gap between the line formed by the anchor points
        /// and the edge of the cabin that lines up with the door.
        gap: f32
    },
    /// The model pose is relative to the center point of the two Lift anchors,
    /// with the y-axis facing the left anchor. The lift doors should open along
    /// the +/- y-axis, and agents should exit the lift along the positive x-axis.
    Model(Model),
}

pub struct Lift<AnchorID> {
    /// These anchors define the canonical reference frame of the lift.
    pub anchors: (AnchorID, AnchorID),
    pub cabin: LiftCabin,
    /// For each level (key of the map, given as its ID in the [`Site`]::levels
    /// map), specify two anchors that correct the positioning of this lift on
    /// that level. These will act like [`Fiducial`] when the site levels are
    /// being aligned.
    ///
    /// Finalized site files should not have this field because it should become
    /// unnecessary after levels have been scaled and aligned.
    pub corrections: BTreeMap<u32, (AnchorID, AnchorID)>,
}
