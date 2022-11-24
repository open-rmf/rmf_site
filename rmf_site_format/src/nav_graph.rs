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
use bevy::prelude::{Component, Bundle, Entity, Query, With};
use serde::{Deserialize, Serialize};
use std::collections::BTreeSet;

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
            color: DisplayColor([1.0, 0.5, 0.3, 1.0]),
            marker: NavGraphMarker,
        }
    }
}

#[derive(Serialize, Deserialize, Debug, Clone)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct DisplayColor(pub [f32; 4]);

/// This component is used by graph elements such as [`Lane`] and [`Location`]
/// to indicate what graphs they can be associated with.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum AssociatedGraphs<T: RefTrait> {
    All,
    Only(BTreeSet<T>),
    AllExcept(BTreeSet<T>),
}

impl<T: RefTrait> Default for AssociatedGraphs<T> {
    fn default() -> Self {
        AssociatedGraphs::All
    }
}

#[cfg(feature = "bevy")]
impl AssociatedGraphs<u32> {
    pub fn to_ecs(
        &self,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> AssociatedGraphs<Entity> {
        match self {
            Self::All => AssociatedGraphs::All,
            Self::Only(set) => AssociatedGraphs::Only(Self::set_to_ecs(set, id_to_entity)),
            Self::AllExcept(set) => AssociatedGraphs::AllExcept(Self::set_to_ecs(set, id_to_entity)),
        }
    }

    fn set_to_ecs(
        set: &BTreeSet<u32>,
        id_to_entity: &std::collections::HashMap<u32, Entity>,
    ) -> BTreeSet<Entity> {
        set.iter().map(|g| id_to_entity.get(g).unwrap().clone()).collect()
    }
}

#[cfg(feature="bevy")]
impl AssociatedGraphs<Entity> {
    pub fn to_u32(
        &self,
        q_nav_graph: &Query<&SiteID, With<NavGraphMarker>>,
    ) -> Result<AssociatedGraphs<u32>, Entity> {
        match self {
            Self::All => Ok(AssociatedGraphs::All),
            Self::Only(set) => Ok(AssociatedGraphs::Only(Self::set_to_u32(set, q_nav_graph)?)),
            Self::AllExcept(set) => Ok(AssociatedGraphs::AllExcept(Self::set_to_u32(set, q_nav_graph)?)),
        }
    }

    fn set_to_u32(
        set: &BTreeSet<Entity>,
        q_nav_graph: &Query<&SiteID, With<NavGraphMarker>>,
    ) -> Result<BTreeSet<u32>, Entity> {
        set.iter().map(|e| {
            q_nav_graph.get(*e)
                .map(|s| s.0)
                .map_err(|_| *e)
        })
        .collect()
    }
}
