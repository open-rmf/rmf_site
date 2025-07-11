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
use bevy::prelude::{Bundle, Component, Deref, DerefMut, Query, Reflect, ReflectComponent, With};
use bevy_ecs::prelude::Entity;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

pub const DEFAULT_NAV_GRAPH_COLORS: [[f32; 3]; 8] = [
    [1.0, 0.5, 0.3],
    [0.6, 1.0, 0.5],
    [0.6, 0.8, 1.0],
    [0.6, 0.2, 0.3],
    [0.1, 0.0, 1.0],
    [0.8, 0.4, 0.5],
    [0.9, 1.0, 0.0],
    [0.7, 0.5, 0.1],
];

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct NavGraph {
    pub name: NameInSite,
    pub color: DisplayColor,
    #[serde(skip)]
    pub marker: NavGraphMarker,
}

#[derive(Clone, Copy, Debug, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct NavGraphMarker;

impl Default for NavGraph {
    fn default() -> Self {
        Self {
            name: NameInSite("<Unnamed>".to_string()),
            color: DisplayColor([1.0, 0.5, 0.3]),
            marker: NavGraphMarker,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct DisplayColor(pub [f32; 3]);

/// This component is used by graph elements such as [`Lane`] and [`Location`]
/// to indicate what graphs they can be associated with.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub enum AssociatedGraphs {
    All,
    Only(BTreeSet<Entity>),
    AllExcept(BTreeSet<Entity>),
}

impl AssociatedGraphs {
    pub fn label(&self) -> &'static str {
        match self {
            Self::All => "All",
            Self::Only(_) => "Only",
            Self::AllExcept(_) => "All Except",
        }
    }

    pub fn all(&self) -> bool {
        matches!(self, Self::All)
    }

    pub fn only(&self) -> Option<&BTreeSet<Entity>> {
        match self {
            Self::Only(set) => Some(set),
            _ => None,
        }
    }

    pub fn all_except(&self) -> Option<&BTreeSet<Entity>> {
        match self {
            Self::AllExcept(set) => Some(set),
            _ => None,
        }
    }

    pub fn includes(&self, e: Entity) -> bool {
        match self {
            Self::All => true,
            Self::Only(set) => set.contains(&e),
            Self::AllExcept(set) => !set.contains(&e),
        }
    }
}

impl Default for AssociatedGraphs {
    fn default() -> Self {
        AssociatedGraphs::All
    }
}

impl AssociatedGraphs {
    pub fn convert(&self, id_map: &HashMap<Entity, Entity>) -> Result<AssociatedGraphs, Entity> {
        let result = match self {
            Self::All => AssociatedGraphs::All,
            Self::Only(set) => AssociatedGraphs::Only(Self::convert_set(set, id_map)?),
            Self::AllExcept(set) => AssociatedGraphs::AllExcept(Self::convert_set(set, id_map)?),
        };
        Ok(result)
    }

    fn convert_set(
        set: &BTreeSet<Entity>,
        id_map: &HashMap<Entity, Entity>,
    ) -> Result<BTreeSet<Entity>, Entity> {
        set.iter()
            .map(|g| id_map.get(g).cloned().ok_or(*g))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallAssociatedGraphs {
    pub only: Option<BTreeSet<Entity>>,
    pub all_except: Option<BTreeSet<Entity>>,
    pub consider: Option<Entity>,
}

impl RecallAssociatedGraphs {
    pub fn assume_only(&self, current: &AssociatedGraphs) -> AssociatedGraphs {
        AssociatedGraphs::Only(
            current
                .only()
                .cloned()
                .unwrap_or(self.only.clone().unwrap_or_default()),
        )
    }

    pub fn assume_all_except(&self, current: &AssociatedGraphs) -> AssociatedGraphs {
        AssociatedGraphs::AllExcept(
            current
                .all_except()
                .cloned()
                .unwrap_or(self.all_except.clone().unwrap_or_default()),
        )
    }
}

impl Default for RecallAssociatedGraphs {
    fn default() -> Self {
        Self {
            only: None,
            all_except: None,
            consider: None,
        }
    }
}

impl Recall for RecallAssociatedGraphs {
    type Source = AssociatedGraphs;

    fn remember(&mut self, source: &Self::Source) {
        match source {
            AssociatedGraphs::All => {}
            AssociatedGraphs::Only(set) => {
                self.only = Some(set.clone());
            }
            AssociatedGraphs::AllExcept(set) => {
                self.all_except = Some(set.clone());
            }
        }
    }
}
