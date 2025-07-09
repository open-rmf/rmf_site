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

/// A modifier property used to describe whether an element is explicitly included
/// or hidden in a scenario.
#[derive(Serialize, Deserialize, Debug, Default, Clone, Copy, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub enum Inclusion {
    Included,
    #[default]
    Hidden,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct InstanceModifier {
    #[serde(default, skip_serializing_if = "is_default")]
    pub pose: Option<Pose>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub visibility: Option<bool>,
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct TaskModifier {
    #[serde(default, skip_serializing_if = "is_default")]
    pub inclusion: Option<Inclusion>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub params: Option<TaskParams>,
}

/// Maps a scenario element entity to its modifier entity, if any
#[derive(Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct ScenarioModifiers<T: RefTrait>(pub HashMap<T, T>);

impl<T: RefTrait> Default for ScenarioModifiers<T> {
    fn default() -> Self {
        Self(HashMap::new())
    }
}

impl<T: RefTrait> ScenarioModifiers<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<ScenarioModifiers<U>, T> {
        let modifiers = self
            .0
            .clone()
            .into_iter()
            .map(|(e_id, m_id)| {
                let converted_e_id = id_map.get(&e_id).cloned().ok_or(e_id)?;
                let converted_m_id = id_map.get(&m_id).cloned().ok_or(m_id)?;
                Ok((converted_e_id, converted_m_id))
            })
            .collect::<Result<_, _>>()?;
        Ok(ScenarioModifiers(modifiers))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct Scenario<T: RefTrait> {
    /// Maps instance entity to InstanceModifier data when saving to file
    #[serde(default, skip_serializing_if = "is_default")]
    pub instances: BTreeMap<T, InstanceModifier>,
    /// Maps task entity to TaskModifier data when saving to file
    #[serde(default, skip_serializing_if = "is_default")]
    pub tasks: BTreeMap<T, TaskModifier>,
    #[serde(flatten)]
    pub properties: ScenarioBundle<T>,
}

impl<T: RefTrait> Scenario<T> {
    pub fn from_name_parent(name: Option<String>, parent: Option<T>) -> Scenario<T> {
        Scenario {
            instances: BTreeMap::new(),
            tasks: BTreeMap::new(),
            properties: ScenarioBundle::new(name, parent),
        }
    }
}

// Create a root scenario without parent
impl<T: RefTrait> Default for Scenario<T> {
    fn default() -> Self {
        Self {
            instances: BTreeMap::new(),
            tasks: BTreeMap::new(),
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
            tasks: self
                .tasks
                .clone()
                .into_iter()
                .map(|(id, task)| {
                    let converted_id = id_map.get(&id).cloned().ok_or(id)?;
                    Ok((converted_id, task))
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
    #[serde(skip)]
    pub scenario_modifiers: ScenarioModifiers<T>,
}

impl<T: RefTrait> ScenarioBundle<T> {
    pub fn new(name: Option<String>, parent: Option<T>) -> ScenarioBundle<T> {
        ScenarioBundle {
            name: NameInSite(name.unwrap_or(DEFAULT_SCENARIO_NAME.to_string())),
            parent_scenario: Affiliation(parent),
            scenario_modifiers: ScenarioModifiers(HashMap::new()),
        }
    }
}

impl<T: RefTrait> Default for ScenarioBundle<T> {
    fn default() -> Self {
        Self {
            name: NameInSite(DEFAULT_SCENARIO_NAME.to_string()),
            parent_scenario: Affiliation::default(),
            scenario_modifiers: ScenarioModifiers::default(),
        }
    }
}

impl<T: RefTrait> ScenarioBundle<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<ScenarioBundle<U>, T> {
        Ok(ScenarioBundle {
            name: self.name.clone(),
            parent_scenario: self.parent_scenario.convert(id_map)?,
            scenario_modifiers: self.scenario_modifiers.convert(id_map)?,
        })
    }
}
