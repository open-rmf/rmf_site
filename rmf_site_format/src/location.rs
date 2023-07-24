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
use bevy::prelude::{Bundle, Component, Deref, DerefMut, Entity};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashSet, HashMap};

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct Location<T: RefTrait> {
    pub anchor: Point<T>,
    #[serde(flatten)]
    pub tags: LocationTags,
    pub name: NameInSite,
    pub graphs: AssociatedGraphs<T>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct LocationTags {
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub charger: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub parking: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub holding: Option<String>,
}

impl LocationTags {
    pub fn is_empty(&self) -> bool {
        self.charger.is_none() && self.parking.is_none() && self.holding.is_none()
    }
}

#[cfg(feature = "bevy")]
impl Location<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> Location<Entity> {
        Location {
            anchor: Point(*id_to_entity.get(&self.anchor).unwrap()),
            tags: self.tags.clone(),
            name: self.name.clone(),
            graphs: self.graphs.to_ecs(id_to_entity),
        }
    }
}

impl<T: RefTrait> From<Point<T>> for Location<T> {
    fn from(anchor: Point<T>) -> Self {
        Self {
            anchor,
            tags: Default::default(),
            name: NameInSite("<Unnamed>".to_string()),
            graphs: AssociatedGraphs::All,
        }
    }
}

#[derive(Default, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallLocationTags {
    pub recall_charger: Option<String>,
    pub recall_parking: Option<String>,
    pub recall_holding: Option<String>,
}

impl Recall for RecallLocationTags {
    type Source = LocationTags;

    fn remember(&mut self, source: &Self::Source) {
        if let Some(charger) = &source.charger {
            self.recall_charger = Some(charger.clone());
        }

        if let Some(parking) = &source.parking {
            self.recall_parking = Some(parking.clone());
        }

        if let Some(holding) = &source.holding {
            self.recall_holding = Some(holding.clone());
        }
    }
}
