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
    site::{Original, Change},
    widgets::{
        inspector::{
            InspectAngle,
        },
    },
};
use rmf_site_format::{
    Pose, Rotation, Angle,
};

pub struct PreviousRotation {
    pub yaw: Option<Angle>,
    pub euler: Option<[Angle; 3]>,
    pub quat: Option<[f32; 4]>,
}

pub struct PreviousPose {
    pub rotation: PreviousRotation,
}

pub struct InspectPose<'a> {
    pub pose: &'a Pose,
    pub previous: &'a PreviousPose,
}
