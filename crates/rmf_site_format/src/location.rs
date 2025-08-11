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
use bevy::prelude::{Bundle, Component, Deref, DerefMut};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum LocationTag {
    Charger,
    ParkingSpot,
    HoldingPoint,
    Workcell(Model),
}

impl LocationTag {
    pub fn label(&self) -> &'static str {
        match self {
            Self::Charger => "Charger",
            Self::ParkingSpot => "Parking Spot",
            Self::HoldingPoint => "Holding Point",
            Self::Workcell(_) => "Workcell",
        }
    }

    pub fn is_charger(&self) -> bool {
        matches!(self, Self::Charger)
    }
    pub fn is_parking_spot(&self) -> bool {
        matches!(self, Self::ParkingSpot)
    }
    pub fn is_holding_point(&self) -> bool {
        matches!(self, Self::HoldingPoint)
    }
    pub fn workcell(&self) -> Option<&Model> {
        match self {
            Self::Workcell(model) => Some(model),
            _ => None,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Location<T: RefTrait> {
    pub anchor: Point<T>,
    pub tags: LocationTags,
    pub name: NameInSite,
    #[serde(default, skip_serializing_if = "is_default")]
    pub mutex: Affiliation<T>,
    pub graphs: AssociatedGraphs<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct LocationTags(pub Vec<LocationTag>);

impl Default for LocationTags {
    fn default() -> Self {
        LocationTags(Vec::new())
    }
}

impl<T: RefTrait> Location<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Location<U>, T> {
        Ok(Location {
            anchor: self.anchor.convert(id_map)?,
            tags: self.tags.clone(),
            name: self.name.clone(),
            mutex: self.mutex.convert(id_map)?,
            graphs: self.graphs.convert(id_map)?,
        })
    }
}

impl<T: RefTrait> From<Point<T>> for Location<T> {
    fn from(anchor: Point<T>) -> Self {
        Self {
            anchor,
            tags: Default::default(),
            name: NameInSite("<Unnamed>".to_string()),
            mutex: Default::default(),
            graphs: AssociatedGraphs::All,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallLocationTags {
    pub robot_asset_source_recall: RecallAssetSource,
    pub robot_asset_source: Option<AssetSource>,
    pub workcell_asset_source_recall: RecallAssetSource,
    pub workcell_asset_source: Option<AssetSource>,
    pub robot_name: Option<NameInSite>,
    pub workcell_name: Option<NameInSite>,
    pub consider_tag: Option<LocationTag>,
    pub consider_tag_asset_source_recall: RecallAssetSource,
}

impl RecallLocationTags {
    pub fn assume_tag(&self, current: &LocationTags) -> LocationTag {
        if let Some(tag) = &self.consider_tag {
            match tag {
                LocationTag::Charger | LocationTag::HoldingPoint | LocationTag::ParkingSpot => {
                    // If the tag to consider is one of these three values, then
                    // only accept it if it does not already exist in the current
                    // tag list.
                    if current.0.iter().find(|t| **t == *tag).is_none() {
                        return tag.clone();
                    }
                }
                _ => return tag.clone(),
            }
        }
        if current.0.iter().find(|t| t.is_charger()).is_none() {
            return LocationTag::Charger;
        }
        if current.0.iter().find(|t| t.is_parking_spot()).is_none() {
            return LocationTag::ParkingSpot;
        }
        self.assume_workcell()
    }
    pub fn assume_workcell(&self) -> LocationTag {
        let model = Model {
            name: self.workcell_name.clone().unwrap_or_default(),
            source: self.workcell_asset_source.clone().unwrap_or_default(),
            ..Default::default()
        };
        LocationTag::Workcell(model)
    }
}

impl Recall for RecallLocationTags {
    type Source = LocationTags;

    fn remember(&mut self, source: &Self::Source) {
        for tag in &source.0 {
            // TODO(MXG): Consider isolating this memory per element
            match tag {
                LocationTag::Workcell(cell) => {
                    self.workcell_asset_source_recall.remember(&cell.source);
                    self.workcell_asset_source = Some(cell.source.clone());
                    self.workcell_name = Some(cell.name.clone());
                }
                _ => {}
            }
        }
    }
}
