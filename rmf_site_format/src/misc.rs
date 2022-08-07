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

use serde::{Serialize, Deserialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Side {
    Left,
    Right,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Angle {
    Deg(f32),
    Rad(f32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Rotation {
    Yaw(Angle),
    EulerExternalXYZ(Angle, Angle, Angle),
    Quat(f32, f32, f32, f32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Pose {
    pub trans: (f32, f32, f32),
    pub rot: Rotation,
}

impl Default for Pose {
    fn default() -> Self {
        Self{
            trans: (0., 0., 0.),
            rot: Rotation::Yaw(Angle::Deg(0.)),
        }
    }
}
