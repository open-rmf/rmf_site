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
use bevy::prelude::{Bundle, Component, Deref, DerefMut, Entity, Query, With};
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
#[cfg_attr(feature = "bevy", derive(Component))]
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
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct DisplayColor(pub [f32; 3]);

/// This component is used by graph elements such as [`Lane`] and [`Location`]
/// to indicate what graphs they can be associated with.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum AssociatedGraphs<T: RefTrait> {
    All,
    Only(BTreeSet<T>),
    AllExcept(BTreeSet<T>),
}

impl<T: RefTrait> AssociatedGraphs<T> {
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

    pub fn only(&self) -> Option<&BTreeSet<T>> {
        match self {
            Self::Only(set) => Some(set),
            _ => None,
        }
    }

    pub fn all_except(&self) -> Option<&BTreeSet<T>> {
        match self {
            Self::AllExcept(set) => Some(set),
            _ => None,
        }
    }

    pub fn includes(&self, e: T) -> bool {
        match self {
            Self::All => true,
            Self::Only(set) => set.contains(&e),
            Self::AllExcept(set) => !set.contains(&e),
        }
    }
}

impl<T: RefTrait> Default for AssociatedGraphs<T> {
    fn default() -> Self {
        AssociatedGraphs::All
    }
}

impl<T: RefTrait> AssociatedGraphs<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<AssociatedGraphs<U>, T> {
        let result = match self {
            Self::All => AssociatedGraphs::All,
            Self::Only(set) => AssociatedGraphs::Only(Self::convert_set(set, id_map)?),
            Self::AllExcept(set) => AssociatedGraphs::AllExcept(Self::convert_set(set, id_map)?),
        };
        Ok(result)
    }

    fn convert_set<U: RefTrait>(
        set: &BTreeSet<T>,
        id_map: &HashMap<T, U>,
    ) -> Result<BTreeSet<U>, T> {
        set.iter()
            .map(|g| id_map.get(g).cloned().ok_or(*g))
            .collect()
    }
}

#[cfg(feature = "bevy")]
impl AssociatedGraphs<Entity> {
    pub fn to_u32(
        &self,
        q_nav_graph: &Query<&SiteID, With<NavGraphMarker>>,
    ) -> Result<AssociatedGraphs<u32>, Entity> {
        match self {
            Self::All => Ok(AssociatedGraphs::All),
            Self::Only(set) => Ok(AssociatedGraphs::Only(Self::set_to_u32(set, q_nav_graph)?)),
            Self::AllExcept(set) => Ok(AssociatedGraphs::AllExcept(Self::set_to_u32(
                set,
                q_nav_graph,
            )?)),
        }
    }

    fn set_to_u32(
        set: &BTreeSet<Entity>,
        q_nav_graph: &Query<&SiteID, With<NavGraphMarker>>,
    ) -> Result<BTreeSet<u32>, Entity> {
        set.iter()
            .map(|e| q_nav_graph.get(*e).map(|s| s.0).map_err(|_| *e))
            .collect()
    }
}

#[derive(Clone, Debug, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallAssociatedGraphs<T: RefTrait> {
    pub only: Option<BTreeSet<T>>,
    pub all_except: Option<BTreeSet<T>>,
    pub consider: Option<T>,
}

impl<T: RefTrait> RecallAssociatedGraphs<T> {
    pub fn assume_only(&self, current: &AssociatedGraphs<T>) -> AssociatedGraphs<T> {
        AssociatedGraphs::Only(
            current
                .only()
                .cloned()
                .unwrap_or(self.only.clone().unwrap_or_default()),
        )
    }

    pub fn assume_all_except(&self, current: &AssociatedGraphs<T>) -> AssociatedGraphs<T> {
        AssociatedGraphs::AllExcept(
            current
                .all_except()
                .cloned()
                .unwrap_or(self.all_except.clone().unwrap_or_default()),
        )
    }
}

impl<T: RefTrait> Default for RecallAssociatedGraphs<T> {
    fn default() -> Self {
        Self {
            only: None,
            all_except: None,
            consider: None,
        }
    }
}

impl<T: RefTrait> Recall for RecallAssociatedGraphs<T> {
    type Source = AssociatedGraphs<T>;

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
