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
use bevy::{
    math::Vec3A,
    prelude::{Bundle, Component, Deref, DerefMut, Entity, Query, With},
    render::primitives::Aabb,
};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

pub const DEFAULT_CABIN_WALL_THICKNESS: f32 = 0.1;
pub const DEFAULT_CABIN_DOOR_THICKNESS: f32 = 0.05;
pub const DEFAULT_CABIN_GAP: f32 = 0.01;
pub const DEFAULT_CABIN_WIDTH: f32 = 1.5;
pub const DEFAULT_CABIN_DEPTH: f32 = 1.65;

#[derive(Serialize, Deserialize, Debug, Clone)]
pub struct Lift<T: RefTrait> {
    /// The cabin doors that the lift cabin has
    pub cabin_doors: BTreeMap<T, LiftCabinDoor>,
    /// Properties that define the lift
    pub properties: LiftProperties<T>,
    /// Anchors that are inside the cabin of the lift and exist in the map of
    /// the cabin's interior.
    pub cabin_anchors: BTreeMap<T, Anchor>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct LiftCabinDoor {
    /// What kind of door is this
    pub kind: DoorType,
    #[serde(skip)]
    pub marker: LiftCabinDoorMarker,
}

#[derive(Clone, Copy, Debug, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct LiftCabinDoorMarker;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct LiftProperties<T: RefTrait> {
    /// Name of this lift. This must be unique within the site.
    pub name: NameInSite,
    /// These anchors define the canonical reference frame of the lift. Both
    /// anchors must be site-wide anchors.
    pub reference_anchors: Edge<T>,
    /// Description of the cabin for the lift.
    pub cabin: LiftCabin<T>,
    /// Descriptions of the doors used at each level
    pub level_doors: LevelDoors<T>,
    /// When this is true, the lift is only for decoration and will not be
    /// responsive during a simulation.
    pub is_static: IsStatic,
    /// What is the initial level for this lift. If nothing is specified, the
    /// lift will start on the lowest level.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub initial_level: InitialLevel<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct InitialLevel<T: RefTrait>(pub Option<T>);

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct LevelDoors<T: RefTrait> {
    /// A map from the ID of a level that this lift can visit to the door(s) that
    /// the lift opens on that level. key: level, value: door. The lift can only
    /// visit levels that are included in this map.
    pub visit: BTreeMap<T, Vec<T>>,

    /// Anchors that define the level door positioning for level doors.
    /// The key of this map is the cabin door ID and the value is a pair of
    /// anchor IDs associated with that cabin door, used to mark the location of
    /// where a level door is (or would be) located.
    pub reference_anchors: BTreeMap<T, Edge<T>>,
}

impl<T: RefTrait> Default for LevelDoors<T> {
    fn default() -> Self {
        Self {
            visit: Default::default(),
            reference_anchors: Default::default(),
        }
    }
}

#[cfg(feature="bevy")]
impl LevelDoors<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> LevelDoors<Entity> {
        LevelDoors {
            visit: self.visit.iter().map(|(level, doors)| {
                (
                    *id_to_entity.get(level).unwrap(),
                    doors.iter().map(|door| id_to_entity.get(door).unwrap()).copied().collect()
                )
            }).collect(),
            reference_anchors: self.reference_anchors.iter().map(|(door, edge)| {
                (
                    *id_to_entity.get(door).unwrap(),
                    edge.to_ecs(id_to_entity)
                )
            }).collect(),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum LiftCabin<T: RefTrait> {
    /// The lift cabin is defined by some parameters.
    Rect(RectangularLiftCabin<T>),
    // TODO(MXG): Support Models as lift cabins
    // The model pose is relative to the center point of the two Lift anchors,
    // with the y-axis facing the left anchor. The lift doors should open along
    // the +/- y-axis, and agents should exit the lift along the positive x-axis.
    // Model(Model),
}

impl<T: RefTrait> Default for LiftCabin<T> {
    fn default() -> Self {
        LiftCabin::Rect(Default::default())
    }
}

/// A lift cabin that is defined entirely by a standard set of parameters.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct RectangularLiftCabin<T: RefTrait> {
    /// How wide is the interior of the cabin, along the axis formed by the
    /// anchor points.
    pub width: f32,
    /// How deep is the cabin, i.e. interior distance from the front wall to
    /// the back wall of the cabin.
    pub depth: f32,
    /// How thick are the walls of the cabin. Default is DEFAULT_CABIN_WALL_THICKNESS.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub wall_thickness: Option<f32>,
    /// How large is the gap between the line formed by the anchor points
    /// and the edge of the cabin that lines up with the door. Default is
    /// 0.01m.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub gap: Option<f32>,
    /// Left (positive) / right (negative) shift of the cabin, off-center
    /// from the anchor points. Default is 0.0m.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub shift: Option<f32>,
    /// The placement of the cabin's front door, if it has one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub front_door: Option<LiftCabinDoorPlacement<T>>,
    /// The placement of the cabin's back door, if it has one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub back_door: Option<LiftCabinDoorPlacement<T>>,
    /// The placement of the cabin's left door, if it has one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub left_door: Option<LiftCabinDoorPlacement<T>>,
    /// The placement of the cabin's right door, if it has one
    #[serde(skip_serializing_if = "Option::is_none")]
    pub right_door: Option<LiftCabinDoorPlacement<T>>,
}

impl<T: RefTrait> Default for RectangularLiftCabin<T> {
    fn default() -> Self {
        Self {
            width: DEFAULT_CABIN_WIDTH,
            depth: DEFAULT_CABIN_DEPTH,
            wall_thickness: None,
            gap: None,
            shift: None,
            front_door: None,
            back_door: None,
            left_door: None,
            right_door: None,
        }
    }
}

impl<T: RefTrait> RectangularLiftCabin<T> {
    pub fn thickness(&self) -> f32 {
        self.wall_thickness.unwrap_or(DEFAULT_CABIN_WALL_THICKNESS)
    }

    pub fn gap(&self) -> f32 {
        self.gap.unwrap_or(DEFAULT_CABIN_GAP)
    }

    pub fn shift(&self) -> f32 {
        self.shift.unwrap_or(0.0)
    }

    pub fn doors(&self) -> [&Option<LiftCabinDoorPlacement<T>>; 4] {
        [
            &self.front_door,
            &self.back_door,
            &self.left_door,
            &self.right_door,
        ]
    }

    pub fn doors_mut(&self) -> [&Option<LiftCabinDoorPlacement<T>>; 4] {
        [
            &self.front_door,
            &self.back_door,
            &self.left_door,
            &self.right_door,
        ]
    }
}

#[cfg(feature = "bevy")]
impl<T: RefTrait> RectangularLiftCabin<T> {
    pub fn aabb(&self) -> Aabb {
        Aabb {
            center: Vec3A::new(
                -self.depth / 2.0 - self.thickness() - self.gap(),
                self.shift(),
                DEFAULT_LEVEL_HEIGHT / 2.0,
            ),
            half_extents: Vec3A::new(
                self.depth / 2.0,
                self.width / 2.0,
                DEFAULT_LEVEL_HEIGHT / 2.0,
            ),
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, PartialEq)]
pub struct LiftCabinDoorPlacement<T: RefTrait> {
    /// Reference to the actual door entity
    pub door: T,
    /// How wide is the lift cabin door
    pub width: f32,
    /// Set the thickness of the door. If set to None, 10cm will be used.
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub thickness: Option<f32>,
    /// Shift the door off-center to the left (positive) or right (negative)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub shifted: Option<f32>,
    /// Use a different gap than the one for the parent LiftCabin
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub custom_gap: Option<f32>,
}

#[cfg(feature = "bevy")]
impl LiftProperties<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> LiftProperties<Entity> {
        LiftProperties {
            name: self.name.clone(),
            reference_anchors: self.reference_anchors.to_ecs(id_to_entity),
            cabin: self.cabin.to_ecs(id_to_entity),
            level_doors: self.level_doors.to_ecs(id_to_entity),
            is_static: self.is_static,
            initial_level: InitialLevel(self.initial_level.map(
                |id| id_to_entity.get(&id).unwrap()
            ).copied()),
        }
    }
}

impl<T: RefTrait> From<Edge<T>> for LiftProperties<T> {
    fn from(edge: Edge<T>) -> Self {
        LiftProperties {
            name: Default::default(),
            reference_anchors: edge,
            cabin: LiftCabin::default(),
            level_doors: Default::default(),
            is_static: Default::default(),
            initial_level: InitialLevel(None),
        }
    }
}

#[cfg(feature = "bevy")]
impl LiftCabin<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> LiftCabin<Entity> {
        match self {
            LiftCabin::Rect(cabin) => LiftCabin::Rect(cabin.to_ecs(id_to_entity)),
        }
    }
}

#[cfg(feature="bevy")]
impl LiftCabin<Entity> {
    pub fn to_u32(
        &self,
        doors: &Query<(&SiteID, &DoorType), With<LiftCabinDoorMarker>>,
    ) -> LiftCabin<u32> {
        match self {
            LiftCabin::Rect(cabin) => LiftCabin::Rect(cabin.to_u32(doors)),
        }
    }
}

#[cfg(feature = "bevy")]
impl RectangularLiftCabin<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> RectangularLiftCabin<Entity> {
        RectangularLiftCabin {
            width: self.width,
            depth: self.depth,
            wall_thickness: self.wall_thickness,
            gap: self.gap,
            shift: self.shift,
            front_door: self.front_door.as_ref().map(|d| d.to_ecs(id_to_entity)),
            back_door: self.back_door.as_ref().map(|d| d.to_ecs(id_to_entity)),
            left_door: self.left_door.as_ref().map(|d| d.to_ecs(id_to_entity)),
            right_door: self.right_door.as_ref().map(|d| d.to_ecs(id_to_entity)),
        }
    }
}

#[cfg(feature = "bevy")]
impl RectangularLiftCabin<Entity> {
    pub fn to_u32(
        &self,
        doors: &Query<(&SiteID, &DoorType), With<LiftCabinDoorMarker>>,
    ) -> RectangularLiftCabin<u32> {
        RectangularLiftCabin {
            width: self.width,
            depth: self.depth,
            wall_thickness: self.wall_thickness,
            gap: self.gap,
            shift: self.shift,
            front_door: self.front_door.as_ref().map(|d| d.to_u32(doors)),
            back_door: self.back_door.as_ref().map(|d| d.to_u32(doors)),
            left_door: self.left_door.as_ref().map(|d| d.to_u32(doors)),
            right_door: self.right_door.as_ref().map(|d| d.to_u32(doors)),
        }
    }
}

#[cfg(feature = "bevy")]
impl LiftCabinDoorPlacement<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> LiftCabinDoorPlacement<Entity> {
        LiftCabinDoorPlacement {
            door: *id_to_entity.get(&self.door).unwrap(),
            width: self.width,
            thickness: self.thickness,
            shifted: self.shifted,
            custom_gap: self.custom_gap,
        }
    }
}

#[cfg(feature = "bevy")]
impl LiftCabinDoorPlacement<Entity> {
    pub fn to_u32(
        &self,
        doors: &Query<(&SiteID, &DoorType), With<LiftCabinDoorMarker>>,
    ) -> LiftCabinDoorPlacement<u32> {
        LiftCabinDoorPlacement {
            door: doors.get(self.door).unwrap().0.0,
            width: self.width,
            thickness: self.thickness,
            shifted: self.shifted,
            custom_gap: self.custom_gap,
        }
    }
}
