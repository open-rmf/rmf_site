/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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
#[cfg(feature = "bevy")]
use bevy::prelude::{Component, Entity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub struct MobileRobot {
    pub properties: MobileRobotProperties,
    pub instances: BTreeMap<u32, MobileRobotInstance>,
}

pub struct MobileRobotProperties {
    pub model_name: NameInSite,
    pub source: AssetSource,
    #[serde(default, skip_serializing_if = "is_default")]
    pub scale: Scale,
    pub kinematics: MobileRobotKinematics,
}

pub enum MobileRobotKinematics {
    DifferentialDrive(DifferentialDrive),
}

pub struct DifferentialDrive {
    pub translational_speed: f32,
    pub rotational_speed: f32,
    pub bidirectional: bool,
}

pub struct MobileRobotInstance<T: RefTrait> {
    pub name: NameInSite,
    pub pose: Pose,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub anchor: Option<Point<T>>,
}
