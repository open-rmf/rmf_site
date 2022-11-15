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

use crate::Recall;
#[cfg(feature = "bevy")]
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

pub const DEFAULT_LEVEL_HEIGHT: f32 = 3.0;

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum Side {
    Left,
    Right,
}

impl Side {
    pub fn label(&self) -> &'static str {
        match self {
            Side::Left => "Left",
            Side::Right => "Right",
        }
    }

    pub fn opposite(&self) -> Side {
        match self {
            Side::Left => Side::Right,
            Side::Right => Side::Left,
        }
    }

    pub fn index(&self) -> usize {
        match self {
            Side::Left => 0,
            Side::Right => 1,
        }
    }

    /// The popular convention for robotics is for "Forward" to be along the
    /// +x axis, which means "Left" is +y and "Right" is -y. To conform with
    /// that convention, this function gives back +1.0 for Left and -1.0 for y.
    pub fn sign(&self) -> f32 {
        match self {
            Side::Left => 1.0,
            Side::Right => -1.0,
        }
    }

    /// When the pivot of a door is on this side, get the angle of the door
    /// when it is closed.
    pub fn pivot_closed_angle(&self) -> Angle {
        Angle::Deg(self.index() as f32 * 180.0 - 90.0)
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

impl std::ops::Mul<f32> for Angle {
    type Output = Angle;
    fn mul(self, rhs: f32) -> Self::Output {
        match self {
            Self::Deg(v) => Self::Deg(rhs * v),
            Self::Rad(v) => Self::Rad(rhs * v),
        }
    }
}

impl std::ops::Mul<Angle> for f32 {
    type Output = Angle;
    fn mul(self, rhs: Angle) -> Self::Output {
        rhs * self
    }
}

impl std::ops::Add for Angle {
    type Output = Angle;
    fn add(self, rhs: Self) -> Self::Output {
        match self {
            Self::Deg(v) => Self::Deg(v + rhs.degrees()),
            Self::Rad(v) => Self::Rad(v + rhs.radians()),
        }
    }
}

impl std::ops::Sub for Angle {
    type Output = Angle;
    fn sub(self, rhs: Self) -> Self::Output {
        match self {
            Self::Deg(v) => Self::Deg(v - rhs.degrees()),
            Self::Rad(v) => Self::Rad(v - rhs.radians()),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub enum Rotation {
    Yaw(Angle),
    EulerExtrinsicXYZ([Angle; 3]),
    Quat([f32; 4]),
}

#[cfg(feature = "bevy")]
impl Rotation {
    pub fn as_yaw(&self) -> Self {
        match self {
            Self::Yaw(_) => self.clone(),
            Self::EulerExtrinsicXYZ([_, _, yaw]) => Self::Yaw(*yaw),
            Self::Quat(quat) => {
                Self::Yaw(Angle::Rad(self.as_bevy_quat().to_euler(EulerRot::ZYX).0))
            }
        }
    }

    pub fn as_euler_extrinsic_xyz(&self) -> Self {
        match self {
            Self::Yaw(yaw) => Self::EulerExtrinsicXYZ([Angle::Deg(0.0), Angle::Deg(0.0), *yaw]),
            Self::EulerExtrinsicXYZ(_) => self.clone(),
            Self::Quat(_) => {
                let (z, y, x) = self.as_bevy_quat().to_euler(EulerRot::ZYX);
                Self::EulerExtrinsicXYZ([Angle::Rad(x), Angle::Rad(y), Angle::Rad(z)])
            }
        }
    }

    pub fn as_quat(&self) -> Self {
        Self::Quat(self.as_bevy_quat().to_array())
    }

    pub fn as_bevy_quat(&self) -> Quat {
        match self {
            Self::Yaw(yaw) => Quat::from_rotation_z(yaw.radians()),
            Self::EulerExtrinsicXYZ([x, y, z]) => {
                Quat::from_euler(EulerRot::ZYX, z.radians(), y.radians(), x.radians())
            }
            Self::Quat(quat) => Quat::from_array(*quat),
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::Yaw(_) => "Yaw",
            Self::EulerExtrinsicXYZ(_) => "Euler Extrinsic XYZ",
            Self::Quat(_) => "Quaternion",
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Pose {
    pub trans: [f32; 3],
    pub rot: Rotation,
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            trans: [0., 0., 0.],
            rot: Rotation::Yaw(Angle::Deg(0.)),
        }
    }
}

#[cfg(feature = "bevy")]
impl Pose {
    pub fn transform(&self) -> Transform {
        Transform {
            translation: self.trans.clone().into(),
            rotation: self.rot.as_bevy_quat(),
            ..default()
        }
    }
}

/// The unique name of the site element within its site.
/// NOTE: We call this `NameInSite` instead of just `Name` because `Name`
/// conflicts with another `Name` defined in `bevy::prelude`.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct NameInSite(pub String);

impl Default for NameInSite {
    fn default() -> Self {
        Self("<Unnamed>".to_string())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct Label(pub Option<String>);

impl Default for Label {
    fn default() -> Self {
        Label(None)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallLabel {
    pub value: Option<String>,
}

impl Recall for RecallLabel {
    type Source = Label;

    fn remember(&mut self, source: &Self::Source) {
        match &source.0 {
            Some(value) => {
                self.value = Some(value.clone());
            }
            None => {
                // Do nothing
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct Kind(pub Option<String>);

impl Default for Kind {
    fn default() -> Self {
        Kind(None)
    }
}

#[derive(Default, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallKind {
    pub value: Option<String>,
}

impl Recall for RecallKind {
    type Source = Kind;

    fn remember(&mut self, source: &Self::Source) {
        match &source.0 {
            Some(value) => {
                self.value = Some(value.clone());
            }
            None => {
                // Do nothing
            }
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct IsStatic(pub bool);

impl Default for IsStatic {
    fn default() -> Self {
        IsStatic(false)
    }
}

/// Marker component for previewable entities
#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct PreviewableMarker;
