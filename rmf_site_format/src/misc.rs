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
use glam::{Quat, Vec2, Vec3};
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

/// Enumeration for the faces of a rectangle. Conventionally:
/// Front: +x
/// Back: -x
/// Left: +y
/// Right: -y
#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
pub enum RectFace {
    Front,
    Back,
    Left,
    Right,
}

impl RectFace {
    pub fn iter_all() -> impl Iterator<Item = RectFace> {
        [Self::Front, Self::Back, Self::Left, Self::Right].into_iter()
    }

    pub fn label(&self) -> &'static str {
        match self {
            Self::Front => "Front",
            Self::Back => "Back",
            Self::Left => "Left",
            Self::Right => "Right",
        }
    }

    /// A vector from the center of the rectangle towards this face.
    pub fn u(&self) -> Vec3 {
        match self {
            Self::Front => Vec3::X,
            Self::Back => Vec3::NEG_X,
            Self::Left => Vec3::Y,
            Self::Right => Vec3::NEG_Y,
        }
    }

    /// A vector from the center of the rectange towards your "left-hand"
    /// direction while looking at this face.
    pub fn v(&self) -> Vec3 {
        match self {
            Self::Front => Vec3::Y,
            Self::Back => Vec3::NEG_Y,
            Self::Left => Vec3::NEG_X,
            Self::Right => Vec3::X,
        }
    }

    pub fn uv(&self) -> (Vec3, Vec3) {
        (self.u(), self.v())
    }

    pub fn uv2(&self) -> (Vec2, Vec2) {
        (self.u().truncate(), self.v().truncate())
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, PartialOrd)]
#[serde(rename_all = "snake_case")]
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

    pub fn match_variant(self, other: Angle) -> Self {
        match other {
            Angle::Deg(_) => Angle::Deg(self.degrees()),
            Angle::Rad(_) => Angle::Rad(self.radians()),
        }
    }

    pub fn is_radians(&self) -> bool {
        matches!(self, Angle::Rad(_))
    }

    pub fn is_degrees(&self) -> bool {
        matches!(self, Angle::Deg(_))
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

impl std::ops::AddAssign for Angle {
    fn add_assign(&mut self, rhs: Self) {
        let result = *self + rhs;
        *self = result;
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

impl std::ops::SubAssign for Angle {
    fn sub_assign(&mut self, rhs: Self) {
        let result = *self - rhs;
        *self = result;
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[serde(rename_all = "snake_case")]
pub enum Rotation {
    Yaw(Angle),
    #[serde(rename = "euler_xyz")]
    EulerExtrinsicXYZ([Angle; 3]),
    Quat([f32; 4]),
}

impl Rotation {
    pub fn apply_yaw(&mut self, delta: Angle) {
        match self {
            Self::Yaw(yaw) => *yaw += delta,
            Self::EulerExtrinsicXYZ([_, _, yaw]) => *yaw += delta,
            Self::Quat(quat) => {
                let q = Quat::from_array(*quat);
                *quat = Quat::from_rotation_z(delta.radians())
                    .mul_quat(q)
                    .to_array();
            }
        }
    }
}

#[cfg(feature = "bevy")]
impl Rotation {
    pub fn as_yaw(&self) -> Self {
        match self {
            Self::Yaw(_) => self.clone(),
            Self::EulerExtrinsicXYZ([_, _, yaw]) => Self::Yaw(*yaw),
            Self::Quat(_) => Self::Yaw(Angle::Rad(self.as_bevy_quat().to_euler(EulerRot::ZYX).0)),
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

impl Default for Rotation {
    fn default() -> Self {
        Rotation::Yaw(Angle::Deg(0.))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Pose {
    pub trans: [f32; 3],
    #[serde(default)]
    pub rot: Rotation,
}

impl Default for Pose {
    fn default() -> Self {
        Self {
            trans: [0., 0., 0.],
            rot: Rotation::default(),
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

    pub fn align_with(&mut self, tf: &Transform) {
        self.trans = tf.translation.into();

        match self.rot {
            Rotation::Yaw(angle) => {
                let (yaw, pitch, roll) = tf.rotation.to_euler(EulerRot::ZYX);
                if pitch != 0.0 || roll != 0.0 {
                    // Automatically switch the representation if the pitch or
                    // roll are no longer 0.0
                    self.rot = Rotation::EulerExtrinsicXYZ([
                        Angle::Rad(roll).match_variant(angle),
                        Angle::Rad(pitch).match_variant(angle),
                        Angle::Rad(yaw).match_variant(angle),
                    ]);
                } else {
                    self.rot = Rotation::Yaw(Angle::Rad(yaw).match_variant(angle));
                }
            }
            Rotation::EulerExtrinsicXYZ([o_roll, o_pitch, o_yaw]) => {
                let (yaw, pitch, roll) = tf.rotation.to_euler(EulerRot::ZYX);
                self.rot = Rotation::EulerExtrinsicXYZ([
                    Angle::Rad(roll).match_variant(o_roll),
                    Angle::Rad(pitch).match_variant(o_pitch),
                    Angle::Rad(yaw).match_variant(o_yaw),
                ]);
            }
            Rotation::Quat(_) => {
                self.rot = Rotation::Quat(tf.rotation.to_array());
            }
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

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
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

/// This component is applied to each site element that gets loaded in order to
/// remember what its original ID within the Site file was.
#[derive(Clone, Copy, Debug)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct SiteID(pub u32);

/// The Pending component indicates that an element is not yet ready to be
/// saved to file. We will filter out these elements while assigning SiteIDs,
/// and that will prevent them from being included while collecting elements
/// into the Site data structure.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Pending;

/// The Original component indicates that an element is being modified but not
/// yet in a state where it can be correctly saved. We should save the original
/// value instead of the apparent current value.
#[derive(Debug, Clone, Copy)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct Original<T>(pub T);
