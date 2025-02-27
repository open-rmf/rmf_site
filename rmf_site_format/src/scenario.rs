/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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
use bevy::prelude::{Bundle, Component, Reflect, ReflectComponent};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct InstanceMarker;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub enum Instance {
    Added(AddedInstance),
    Inherited(InheritedInstance),
    Hidden(HiddenInstance),
}

impl Instance {
    pub fn new_added(pose: Pose) -> Self {
        Self::Added(AddedInstance { pose: pose })
    }

    pub fn new_inherited(pose: Option<Pose>) -> Self {
        Self::Inherited(InheritedInstance {
            modified_pose: pose,
        })
    }

    pub fn new_hidden(pose: Option<Pose>) -> Self {
        Self::Hidden(HiddenInstance { pose: pose })
    }

    pub fn pose(&self) -> Option<Pose> {
        match self {
            Instance::Added(added) => Some(added.pose.clone()),
            Instance::Inherited(inherited) => inherited.modified_pose.clone(),
            Instance::Hidden(hidden) => hidden.pose.clone(),
        }
    }
}

/// The instance was added by this scenario
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct AddedInstance {
    pub pose: Pose,
}

/// The instance was inherited from a parent scenario
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct InheritedInstance {
    pub modified_pose: Option<Pose>,
}

/// The instance doesn't exist in this scenario
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct HiddenInstance {
    /// This is the pose of the instance before it was hidden if it was added or modified
    /// Set to None if the instance pose was inherited without modification
    pub pose: Option<Pose>,
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct ScenarioMarker;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Scenario<T: RefTrait> {
    pub parent_scenario: Affiliation<T>,
    pub instances: BTreeMap<T, Instance>,
}

impl<T: RefTrait> Scenario<T> {
    pub fn from_parent(parent: T) -> Scenario<T> {
        Scenario {
            parent_scenario: Affiliation(Some(parent)),
            instances: BTreeMap::new(),
        }
    }
}

// Create a root scenario without parent
impl<T: RefTrait> Default for Scenario<T> {
    fn default() -> Self {
        Self {
            parent_scenario: Affiliation::default(),
            instances: BTreeMap::new(),
        }
    }
}

impl<T: RefTrait> Scenario<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Scenario<U>, T> {
        Ok(Scenario {
            parent_scenario: self.parent_scenario.convert(id_map)?,
            instances: self
                .instances
                .clone()
                .into_iter()
                .map(|(id, instance)| {
                    let converted_id = id_map.get(&id).cloned().ok_or(id)?;
                    Ok((converted_id, instance))
                })
                .collect::<Result<_, _>>()?,
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct ScenarioBundle<T: RefTrait> {
    pub name: NameInSite,
    pub scenario: Scenario<T>,
    pub marker: ScenarioMarker,
}

impl<T: RefTrait> ScenarioBundle<T> {
    pub fn from_name_parent(
        name: String,
        parent: Option<T>,
        instances: &BTreeMap<T, Instance>,
    ) -> ScenarioBundle<T> {
        ScenarioBundle {
            name: NameInSite(name),
            scenario: Scenario {
                parent_scenario: Affiliation(parent),
                instances: instances.clone(),
            },
            marker: ScenarioMarker,
        }
    }
}

impl<T: RefTrait> Default for ScenarioBundle<T> {
    fn default() -> Self {
        Self {
            name: NameInSite("Default Scenario".to_string()),
            scenario: Scenario::default(),
            marker: ScenarioMarker,
        }
    }
}

impl<T: RefTrait> ScenarioBundle<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<ScenarioBundle<U>, T> {
        Ok(ScenarioBundle {
            name: self.name.clone(),
            scenario: self.scenario.convert(id_map)?,
            marker: ScenarioMarker,
        })
    }
}
