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
use bevy::prelude::{Bundle, Component, Deref, DerefMut, Reflect, ReflectComponent};
use serde::{Deserialize, Serialize};
use std::collections::{BTreeMap, HashMap};

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct InstanceMarker;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum InstanceModifier {
    Added(AddedInstance),
    Inherited(InheritedInstance),
    Hidden,
}

impl Default for InstanceModifier {
    fn default() -> Self {
        Self::Hidden
    }
}

impl InstanceModifier {
    pub fn added(pose: Pose) -> Self {
        Self::Added(AddedInstance { pose: pose })
    }

    pub fn inherited() -> Self {
        Self::Inherited(InheritedInstance {
            modified_pose: None,
            explicit_inclusion: false,
        })
    }

    pub fn pose(&self) -> Option<Pose> {
        match self {
            InstanceModifier::Added(added) => Some(added.pose.clone()),
            InstanceModifier::Inherited(inherited) => inherited.modified_pose.clone(),
            InstanceModifier::Hidden => None,
        }
    }

    pub fn visibility(&self) -> Option<bool> {
        match self {
            InstanceModifier::Added(_) => Some(true),
            InstanceModifier::Inherited(inherited) => {
                if inherited.explicit_inclusion {
                    Some(true)
                } else {
                    None
                }
            }
            InstanceModifier::Hidden => Some(false),
        }
    }
}

#[derive(Clone, Debug, Default)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct RecallInstance {
    pub pose: Option<Pose>,
    pub modifier: Option<InstanceModifier>,
}

impl Recall for RecallInstance {
    type Source = InstanceModifier;

    fn remember(&mut self, source: &InstanceModifier) {
        match source {
            InstanceModifier::Added(_) | InstanceModifier::Inherited(_) => {
                self.pose = source.pose();
                self.modifier = Some(source.clone());
            }
            InstanceModifier::Hidden => {
                // We don't update the pose if this InstanceModifier is hidden
            }
        };
    }
}

/// The instance modifier was added by this scenario
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct AddedInstance {
    pub pose: Pose,
}

/// The instance modifier was inherited from a parent scenario
#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
pub struct InheritedInstance {
    pub modified_pose: Option<Pose>,
    pub explicit_inclusion: bool,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct ScenarioModifiers<T: RefTrait>(pub HashMap<T, T>);

impl<T: RefTrait> Default for ScenarioModifiers<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<T: RefTrait> ScenarioModifiers<T> {
    pub fn get_modifiers(&self, entity: T) -> Option<T> {
        self.0.get(&entity).copied()
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default, PartialEq, Eq)]
#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
pub struct ScenarioMarker;

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Scenario<T: RefTrait> {
    pub instances: BTreeMap<T, InstanceModifier>,
    #[serde(flatten)]
    pub properties: ScenarioBundle<T>,
}

impl<T: RefTrait> Scenario<T> {
    pub fn from_name_parent(name: Option<String>, parent: Option<T>) -> Scenario<T> {
        Scenario {
            instances: BTreeMap::new(),
            properties: ScenarioBundle::new(name, parent),
        }
    }
}

// Create a root scenario without parent
impl<T: RefTrait> Default for Scenario<T> {
    fn default() -> Self {
        Self {
            instances: BTreeMap::new(),
            properties: ScenarioBundle::default(),
        }
    }
}

impl<T: RefTrait> Scenario<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Scenario<U>, T> {
        Ok(Scenario {
            instances: self
                .instances
                .clone()
                .into_iter()
                .map(|(id, instance)| {
                    let converted_id = id_map.get(&id).cloned().ok_or(id)?;
                    Ok((converted_id, instance))
                })
                .collect::<Result<_, _>>()?,
            properties: self.properties.convert(id_map)?,
        })
    }
}

const DEFAULT_SCENARIO_NAME: &'static str = "Default Scenario";

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct ScenarioBundle<T: RefTrait> {
    pub name: NameInSite,
    pub parent_scenario: Affiliation<T>,
    pub marker: ScenarioMarker,
}

impl<T: RefTrait> ScenarioBundle<T> {
    pub fn new(name: Option<String>, parent: Option<T>) -> ScenarioBundle<T> {
        ScenarioBundle {
            name: NameInSite(name.unwrap_or(DEFAULT_SCENARIO_NAME.to_string())),
            parent_scenario: Affiliation(parent),
            marker: ScenarioMarker,
        }
    }
}

impl<T: RefTrait> Default for ScenarioBundle<T> {
    fn default() -> Self {
        Self {
            name: NameInSite(DEFAULT_SCENARIO_NAME.to_string()),
            parent_scenario: Affiliation::default(),
            marker: ScenarioMarker,
        }
    }
}

impl<T: RefTrait> ScenarioBundle<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<ScenarioBundle<U>, T> {
        Ok(ScenarioBundle {
            name: self.name.clone(),
            parent_scenario: self.parent_scenario.convert(id_map)?,
            marker: ScenarioMarker,
        })
    }
}
