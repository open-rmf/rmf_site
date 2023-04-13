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
use bevy::prelude::{Bundle, Component, Entity};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Passage<T: RefTrait> {
    /// The endpoints of the passage (start, end)
    pub anchors: Edge<T>,
    /// How the passage is aligned relative to the anchors
    pub alignment: PassageAlignment,
    /// Description of the cells within the passage.
    pub cells: PassageCells,
    /// What graphs this passage is associated with
    pub graphs: AssociatedGraphs<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct PassageAlignment {
    /// Shift the passage longitudinally, along the line formed by the anchors.
    #[serde(default, skip_serializing_if="is_zero")]
    pub longitudinal: f32,
    /// Align the passage left, right, or center of the lane, optionally with an
    /// offset.
    pub lateral: PassageLateralAlignment,
}

#[derive(Serialize, Deserialize, Debug, Clone)]
pub enum PassageLateralAlignment {
    Center(f32),
    Left(f32),
    Right(f32),
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct PassageCells {
    /// How many lanes does this passage have? Each lane of cells runs from the
    /// first anchor to the second anchor, offset from each other laterally.
    /// Their lateral alignment with the anchors is determined by the
    /// [`PassageAlignment`] component in the [`Passage`] bundle.
    pub lanes: usize,
    /// What is the length/width of each cell. Passage cells are always sqaure,
    /// so this size applies equally to both the length and the width.
    pub cell_size: f32,
    /// How many rows of cells should overflow at the first and second anchor
    /// respectively. These value may be negative, in which case rows will be
    /// removed from the ends of the passage.
    #[serde(default, skip_serializing_if="has_zero_overflow")]
    pub overflow: [i32; 2],
    /// The default constraints for the cells in this passage
    #[serde(default, skip_serializing_if="CellConstraints::is_default")]
    pub default_constraints: CellConstraints,
    /// The constraints for each individual cell in the passage.
    #[serde(default, skip_serializing_if="BTreeMap::is_empty")]
    pub constraints: BTreeMap<[i32; 2], CellConstraints>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CellConstraints {
    #[serde(default, skip_serializing_if="CellTransition::is_unconstrained")]
    pub forward: CellTransition,
    #[serde(default, skip_serializing_if="CellTransition::is_unconstrained")]
    pub backward: CellTransition,
    #[serde(default, skip_serializing_if="CellTransition::is_unconstrained")]
    pub left: CellTransition,
    #[serde(default, skip_serializing_if="CellTransition::is_unconstrained")]
    pub right: CellTransition,
}

impl Default for CellConstraints {
    fn default() -> Self {
        Self {
            forward: CellTransition::Unconstrained,
            backward: CellTransition::Disabled,
            left: CellTransition::Disabled,
            right: CellTransition::Disabled,
        }
    }
}

impl CellConstraints {
    pub fn is_default(&self) -> bool {
        self.forward.is_unconstrained()
        && self.backward.is_unconstrained()
        && self.left.is_unconstrained()
        && self.right.is_unconstrained()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub enum CellTransition {
    #[default]
    Unconstrained,
    Constrained(CellTransitionConstraint),
    Disabled,
}

impl CellTransition {
    pub fn is_unconstrained(&self) -> bool {
        matches!(self, CellTransition::Unconstrained)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CellTransitionConstraint {
    /// The speed limit for agents making this cell transition.
    pub speed_limit: Option<f32>,
    /// Cost modifier for choosing this cell transition.
    pub penalty: Option<CellTransitionPenalty>,
    /// What kind of orientation the agent must have while doing this transition.
    #[serde(default, skip_serializing_if="OrientationConstraint::is_none")]
    pub orientation: OrientationConstraint,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct CellTransitionPenalty {
    /// Multiply the ordinary cost of the cell transition by this factor. It is
    /// recommended to only use values greater than or equal to 1.0.
    #[serde(default="float_one", skip_serializing_if="is_one")]
    pub multiplier: f32,
    /// Add this additional cost to the ordinary cost of the cell transition. It
    /// is recommended to only use values greater than or equal to 0.0.
    ///
    /// The `multiplier` value will not be applied to this additional cost.
    #[serde(default="float_zero", skip_serializing_if="is_zero")]
    pub addition: f32,
}

#[cfg(feature = "bevy")]
impl Passage<u32> {
    pub fn to_ecs(&self, id_to_entity: &std::collections::HashMap<u32, Entity>) -> Passage<Entity> {
        Passage {
            anchors: self.anchors.to_ecs(id_to_entity),
            alignment: self.alignment.clone(),
            cells: self.cells.clone(),
            graphs: self.graphs.to_ecs(id_to_entity),
        }
    }
}

impl<T: RefTrait> From<Edge<T>> for Passage<T> {
    fn from(edge: Edge<T>) -> Self {
        Self {
            anchors: edge,
            alignment: PassageAlignment {
                longitudinal: 0.0,
                lateral: PassageLateralAlignment::Left(0.0),
            },
            cells: PassageCells {
                lanes: 1,
                cell_size: 0.5,
                overflow: [0, 0],
                default_constraints: CellConstraints::default(),
                constraints: BTreeMap::new(),
            },
            graphs: Default::default(),
        }
    }
}
