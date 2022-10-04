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
#[cfg(feature = "bevy")]
use bevy::prelude::{Bundle, Component, Entity};
use serde::{Deserialize, Serialize};

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Door<T: RefTrait> {
    /// (left_anchor, right_anchor)
    pub anchors: Edge<T>,
    /// Name of the door. RMF requires each door name to be unique among all
    /// doors in the site.
    pub name: NameInSite,
    /// What kind of door is it.
    pub kind: DoorType,
    #[serde(skip)]
    pub marker: DoorMarker,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum DoorType {
    SingleSliding(SingleSlidingDoor),
    DoubleSliding(DoubleSlidingDoor),
    SingleSwing(SingleSwingDoor),
    DoubleSwing(DoubleSwingDoor),
    /// A custom model for the door. The reference frame for the model will be
    /// the center point of the two anchor points with the y axis facing the
    /// left anchor point and the z axis pointing upwards. The model must have
    /// its own door control plugin.
    Model(Model),
}

impl DoorType {
    pub fn single_sliding(&self) -> Option<&SingleSlidingDoor> {
        match self {
            Self::SingleSliding(v) => Some(v),
            _ => None,
        }
    }

    pub fn double_sliding(&self) -> Option<&DoubleSlidingDoor> {
        match self {
            Self::DoubleSliding(v) => Some(v),
            _ => None,
        }
    }

    pub fn single_swing(&self) -> Option<&SingleSwingDoor> {
        match self {
            Self::SingleSwing(v) => Some(v),
            _ => None,
        }
    }

    pub fn double_swing(&self) -> Option<&DoubleSwingDoor> {
        match self {
            Self::DoubleSwing(v) => Some(v),
            _ => None,
        }
    }

    pub fn model(&self) -> Option<&Model> {
        match self {
            Self::Model(v) => Some(v),
            _ => None,
        }
    }

    pub fn label(&self) -> &str {
        match self {
            Self::SingleSliding(_) => "Single Sliding",
            Self::DoubleSliding { .. } => "Double Sliding",
            Self::SingleSwing { .. } => "Single Swing",
            Self::DoubleSwing(_) => "Double Swing",
            Self::Model(_) => "Model",
        }
    }
}

/// A single door that slides to one side
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SingleSlidingDoor {
    /// Which side the door slides towards
    pub towards: Side,
}

impl Default for SingleSlidingDoor {
    fn default() -> Self {
        Self {
            towards: Side::Left,
        }
    }
}

impl From<SingleSlidingDoor> for DoorType {
    fn from(v: SingleSlidingDoor) -> Self {
        Self::SingleSliding(v)
    }
}

/// Two doors that each slide towards their own side
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DoubleSlidingDoor {
    /// Length of the left door divided by the length of the right door
    pub left_right_ratio: f32,
}

impl Default for DoubleSlidingDoor {
    fn default() -> Self {
        Self {
            left_right_ratio: 1.0,
        }
    }
}

impl From<DoubleSlidingDoor> for DoorType {
    fn from(v: DoubleSlidingDoor) -> Self {
        Self::DoubleSliding(v)
    }
}

/// A single door that swings along a pivot on the left or right side
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct SingleSwingDoor {
    /// Which anchor (Left or Right) does the door pivot on
    pub pivot_on: Side,
    /// How does the door swing
    pub swing: Swing,
}

impl Default for SingleSwingDoor {
    fn default() -> Self {
        Self {
            pivot_on: Side::Left,
            swing: Swing::Forward(Angle::Deg(90.0)),
        }
    }
}

impl From<SingleSwingDoor> for DoorType {
    fn from(v: SingleSwingDoor) -> Self {
        Self::SingleSwing(v)
    }
}

/// Two doors, one left and one right, that each swing on their own pivot.
/// It is assumed their swinging parameters are symmetrical.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct DoubleSwingDoor {
    pub swing: Swing,
}

impl Default for DoubleSwingDoor {
    fn default() -> Self {
        Self {
            swing: Swing::Forward(Angle::Deg(90.0)),
        }
    }
}

impl From<DoubleSwingDoor> for DoorType {
    fn from(v: DoubleSwingDoor) -> Self {
        Self::DoubleSwing(v)
    }
}

impl From<Model> for DoorType {
    fn from(v: Model) -> Self {
        Self::Model(v)
    }
}

/// How the door swings relative to someone who is standing in the frame of door
/// with the left and right sides of their body aligned with the left and right
/// anchor points of the door.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Swing {
    /// Swing forwards up to this (positive) angle
    Forward(Angle),
    /// Swing backwards up to this (positive) angle
    Backward(Angle),
    /// Swing each direction by (forward, backward) positive degrees.
    Both(Angle, Angle),
}

#[derive(Clone, Copy, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct DoorMarker;

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallDoorType {
    pub single_sliding: Option<DoorType>,
    pub double_sliding: Option<DoorType>,
    pub single_swing: Option<DoorType>,
    pub double_swing: Option<DoorType>,
    pub model: Option<DoorType>,
}

impl RecallDoorType {
    pub fn assume_single_sliding(&self, current: &DoorType) -> DoorType {
        current
            .single_sliding()
            .map(|x| x.clone().into())
            .unwrap_or(
                self.single_sliding
                    .as_ref()
                    .map(|x| x.clone())
                    .unwrap_or(DoorType::SingleSliding(SingleSlidingDoor::default())),
            )
    }

    pub fn assume_double_sliding(&self, current: &DoorType) -> DoorType {
        current
            .double_sliding()
            .map(|x| x.clone().into())
            .unwrap_or(
                self.double_sliding
                    .as_ref()
                    .map(|x| x.clone())
                    .unwrap_or(DoorType::DoubleSliding(DoubleSlidingDoor::default())),
            )
    }

    pub fn assume_single_swing(&self, current: &DoorType) -> DoorType {
        current.single_swing().map(|x| x.clone().into()).unwrap_or(
            self.single_swing
                .as_ref()
                .map(|x| x.clone())
                .unwrap_or(DoorType::SingleSwing(SingleSwingDoor::default())),
        )
    }

    pub fn assume_double_swing(&self, current: &DoorType) -> DoorType {
        current.double_swing().map(|x| x.clone().into()).unwrap_or(
            self.double_swing
                .as_ref()
                .map(|x| x.clone())
                .unwrap_or(DoorType::DoubleSwing(DoubleSwingDoor::default())),
        )
    }

    pub fn assume_model(&self, current: &DoorType) -> DoorType {
        current.model().map(|x| x.clone().into()).unwrap_or(
            self.model
                .as_ref()
                .map(|x| x.clone())
                .unwrap_or(DoorType::Model(Model::default())),
        )
    }
}

impl Recall for RecallDoorType {
    type Source = DoorType;

    fn remember(&mut self, source: &Self::Source) {
        match source {
            DoorType::SingleSliding(_) => {
                self.single_sliding = Some(source.clone());
            }
            DoorType::DoubleSliding { .. } => {
                self.double_sliding = Some(source.clone());
            }
            DoorType::SingleSwing { .. } => {
                self.single_sliding = Some(source.clone());
            }
            DoorType::DoubleSwing(_) => {
                self.double_swing = Some(source.clone());
            }
            DoorType::Model(_) => {
                self.model = Some(source.clone());
            }
        }
    }
}

#[cfg(feature = "bevy")]
impl Door<Entity> {
    pub fn to_u32(&self, anchors: Edge<u32>) -> Door<u32> {
        Door {
            anchors,
            name: self.name.clone(),
            kind: self.kind.clone(),
            marker: Default::default(),
        }
    }
}

#[cfg(feature = "bevy")]
impl Door<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Door<Entity> {
        Door {
            anchors: self.anchors.to_ecs(id_to_entity),
            name: self.name.clone(),
            kind: self.kind.clone(),
            marker: Default::default(),
        }
    }
}

impl<T: RefTrait> From<Edge<T>> for Door<T> {
    fn from(edge: Edge<T>) -> Self {
        Door {
            anchors: edge,
            name: NameInSite("<Unnamed>".to_string()),
            kind: SingleSlidingDoor::default().into(),
            marker: Default::default(),
        }
    }
}
