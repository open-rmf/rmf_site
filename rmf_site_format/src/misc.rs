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
#[cfg(feature="bevy")]
use bevy::prelude::*;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum Side {
    Left,
    Right,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Angle {
    Deg(f32),
    Rad(f32),
}

impl Angle {
    pub fn radians(&self) -> f32 {
        match self {
            Angle::Deg(v) => v.to_radians(),
            Angle::Rad(v) => *v,
        }
    }

    pub fn degrees(&self) -> f32 {
        match self {
            Angle::Deg(v) => *v,
            Angle::Rad(v) => v.to_degrees(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Rotation {
    Yaw(Angle),
    EulerExtrinsicXYZ(Angle, Angle, Angle),
    Quat(f32, f32, f32, f32),
}

#[cfg(feature="bevy")]
impl Rotation {
    pub fn quat(&self) -> Quat {
        match self {
            Self::Yaw(yaw) => Quat::from_rotation_z(yaw.radians()),
            Self::EulerExtrinsicXYZ(x, y, z) => {
                Quat::from_euler(
                    EulerRot::ZYX, z.radians(), y.radians(), x.radians()
                )
            },
            Self::Quat(x, y, z, w) => {
                Quat::from_array([*x, *y, *z, *w])
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
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

#[cfg(feature="bevy")]
impl Pose {
    pub fn transform(&self) -> Transform {
        Transform{
            translation: self.trans.clone().into(),
            rotation: self.rot.quat(),
            ..default()
        }
    }
}
