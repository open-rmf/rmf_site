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

pub enum OrientationConstraint {
    Forward,
    Reverse,
    RelativeYaw(f32),
    AbsoluteYaw(f32),
}

pub struct LaneProperties {
    pub orientation_constraint: Option<OrientationConstraint>,
    pub speed_limit: Option<f32>,
    pub dock: Option<Dock>,
}

pub enum ReverseLane {
    Same,
    Disable,
    Different(LaneProperties),
}

pub struct Lane<AnchorID> {
    /// The endpoints of the lane (start, end)
    pub anchors: (AnchorID, AnchorID),
    /// The properties of the lane when traveling forwards
    pub forward: LaneProperties,
    /// The properties of the lane when traveling in reverse
    pub reverse: ReverseLane,
}
