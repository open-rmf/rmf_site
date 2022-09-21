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

pub const DEFAULT_LEVEL_HEIGHT: f32 = 3.0;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn opposite(&self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd)]
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
#[cfg_attr(feature="bevy", derive(Component))]
pub struct Pose {
    pub trans: [f32; 3],
    pub rot: Rotation,
}

impl Default for Pose {
    fn default() -> Self {
        Self{
            trans: [0., 0., 0.],
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

/// The unique name of the site element within its site.
/// NOTE: We call this `NameInSite` instead of just `Name` because `Name`
/// conflicts with another `Name` defined in `bevy::prelude`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature="bevy", derive(Component, Deref, DerefMut))]
pub struct NameInSite(pub String);

impl Default for NameInSite {
    fn default() -> Self {
        Self("<Unnamed>".to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature="bevy", derive(Component, Deref, DerefMut))]
pub struct Label(pub Option<String>);

impl Default for Label {
    fn default() -> Self {
        Label(None)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature="bevy", derive(Component, Deref, DerefMut))]
pub struct IsStatic(pub bool);

impl Default for IsStatic {
    fn default() -> Self {
        IsStatic(false)
    }
}
