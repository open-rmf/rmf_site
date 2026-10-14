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
    pub inclusion: Option<Inclusion>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub on_level: Option<u32>,
}

impl InstanceModifier {
    /// Check if this modifier is just the default value and therefore does not
    /// need to be saved.
    pub fn is_default(&self) -> bool {
        is_default(self)
    }
}

#[derive(Serialize, Deserialize, Debug, Default, Clone, PartialEq)]
pub struct TaskModifier {
    #[serde(default, skip_serializing_if = "is_default")]
    pub inclusion: Option<Inclusion>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub params: Option<TaskParams>,
}

impl TaskModifier {
    /// Check if this modifier is just the default value and therefore does not
    /// need to be saved.
    pub fn is_default(&self) -> bool {
        is_default(self)
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LaneModifier<T: RefTrait> {
    #[serde(default, skip_serializing_if = "is_default")]
    pub inclusion: Option<Inclusion>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub forward: Option<Motion>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub reverse: Option<ReverseLane>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub graphs: Option<AssociatedGraphs<T>>,
}

impl<T: RefTrait> Default for LaneModifier<T> {
    fn default() -> Self {
        Self {
            inclusion: None,
            forward: None,
            reverse: None,
            graphs: None,
        }
    }
}

impl<T: RefTrait> LaneModifier<T> {
    /// Check if this modifier is just the default value and therefore does not
    /// need to be saved.
    pub fn is_default(&self) -> bool {
        is_default(self)
    }

    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<LaneModifier<U>, T> {
        Ok(LaneModifier {
            inclusion: self.inclusion,
            forward: self.forward.clone(),
            reverse: self.reverse.clone(),
            graphs: match &self.graphs {
                Some(graphs) => Some(graphs.convert(id_map)?),
                None => None,
            },
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct LocationModifier<T: RefTrait> {
    #[serde(default, skip_serializing_if = "is_default")]
    pub inclusion: Option<Inclusion>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub tags: Option<LocationTags>,
    #[serde(default, skip_serializing_if = "is_default")]
    pub graphs: Option<AssociatedGraphs<T>>,
}

impl<T: RefTrait> Default for LocationModifier<T> {
    fn default() -> Self {
        Self {
            inclusion: None,
            tags: None,
            graphs: None,
        }
    }
}

impl<T: RefTrait> LocationModifier<T> {
    /// Check if this modifier is just the default value and therefore does not
    /// need to be saved.
    pub fn is_default(&self) -> bool {
        is_default(self)
    }

    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<LocationModifier<U>, T> {
        Ok(LocationModifier {
            inclusion: self.inclusion,
            tags: self.tags.clone(),
            graphs: match &self.graphs {
                Some(graphs) => Some(graphs.convert(id_map)?),
                None => None,
            },
        })
    }
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
    /// Maps lane entity to LaneModifier data when saving to file
    #[serde(default, skip_serializing_if = "is_default")]
    pub lanes: BTreeMap<T, LaneModifier<T>>,
    /// Maps location entity to LocationModifier data when saving to file
    #[serde(default, skip_serializing_if = "is_default")]
    pub locations: BTreeMap<T, LocationModifier<T>>,
    #[serde(flatten)]
    pub properties: ScenarioBundle<T>,
}

impl<T: RefTrait> Scenario<T> {
    pub fn from_name_parent(name: Option<String>, parent: Option<T>) -> Scenario<T> {
        Scenario {
            instances: BTreeMap::new(),
            tasks: BTreeMap::new(),
            lanes: BTreeMap::new(),
            locations: BTreeMap::new(),
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
            lanes: BTreeMap::new(),
            locations: BTreeMap::new(),
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
            lanes: self
                .lanes
                .iter()
                .map(|(id, lane)| {
                    let converted_id = id_map.get(id).cloned().ok_or(*id)?;
                    Ok((converted_id, lane.convert(id_map)?))
                })
                .collect::<Result<_, _>>()?,
            locations: self
                .locations
                .iter()
                .map(|(id, location)| {
                    let converted_id = id_map.get(id).cloned().ok_or(*id)?;
                    Ok((converted_id, location.convert(id_map)?))
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

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::BTreeSet;

    #[test]
    fn test_scenario_nav_graph_modifiers_serde_and_convert() {
        let default_scenario = Scenario::<u32>::default();
        let json = serde_json::to_value(&default_scenario).unwrap();
        assert!(json.get("lanes").is_none());
        assert!(json.get("locations").is_none());
        assert!(LaneModifier::<u32>::default().is_default());
        assert!(LocationModifier::<u32>::default().is_default());

        let mut scenario = Scenario::<u32>::from_name_parent(Some("Traffic Test".into()), Some(1));
        scenario.lanes.insert(
            10,
            LaneModifier {
                inclusion: Some(Inclusion::Hidden),
                forward: Some(Motion {
                    orientation_constraint: OrientationConstraint::Forwards,
                    speed_limit: Some(1.5),
                    dock: None,
                }),
                reverse: Some(ReverseLane::Disable),
                graphs: Some(AssociatedGraphs::Only(BTreeSet::from([100]))),
            },
        );
        scenario.locations.insert(
            20,
            LocationModifier {
                inclusion: Some(Inclusion::Included),
                tags: Some(LocationTags(vec![LocationTag::Charger])),
                graphs: Some(AssociatedGraphs::AllExcept(BTreeSet::from([100]))),
            },
        );

        let serialized = serde_json::to_string(&scenario).unwrap();
        let roundtripped: Scenario<u32> = serde_json::from_str(&serialized).unwrap();
        assert_eq!(scenario, roundtripped);

        let id_map = HashMap::from([(1u32, 1001u32), (10, 1010), (20, 1020), (100, 1100)]);
        let converted = scenario.convert(&id_map).unwrap();
        assert_eq!(converted.properties.parent_scenario, Affiliation(Some(1001)));
        assert_eq!(
            converted.lanes.get(&1010).unwrap().graphs,
            Some(AssociatedGraphs::Only(BTreeSet::from([1100])))
        );
        assert_eq!(
            converted.locations.get(&1020).unwrap().graphs,
            Some(AssociatedGraphs::AllExcept(BTreeSet::from([1100])))
        );
    }
}
