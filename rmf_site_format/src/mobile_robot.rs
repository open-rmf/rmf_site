/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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
use bevy::prelude::{Bundle, Component};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct MobileRobot {
    pub model_name: NameInSite,
    pub source: AssetSource,
    #[serde(default, skip_serializing_if = "is_default")]
    pub scale: Scale,
    pub kinematics: MobileRobotKinematics,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum MobileRobotKinematics {
    DifferentialDrive(DifferentialDrive),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct DifferentialDrive {
    pub translational_speed: f32,
    pub rotational_speed: f32,
    pub bidirectional: bool,
}