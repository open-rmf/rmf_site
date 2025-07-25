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
use std::{
    collections::{BTreeMap, BTreeSet, HashMap},
    hash::Hash,
    io,
};
use uuid::Uuid;

pub use ron::ser::PrettyConfig as Style;

#[derive(Serialize, Deserialize, Debug, Clone)]
#[cfg_attr(feature = "bevy", derive(Bundle))]
pub struct SiteProperties<T: RefTrait> {
    pub name: NameOfSite,
    #[serde(skip_serializing_if = "GeographicComponent::is_none")]
    pub geographic_offset: GeographicComponent,
    // TODO(luca) group these into an IssueFilters?
    #[serde(default, skip_serializing_if = "FilteredIssues::is_empty")]
    pub filtered_issues: FilteredIssues<T>,
    #[serde(default, skip_serializing_if = "FilteredIssueKinds::is_empty")]
    pub filtered_issue_kinds: FilteredIssueKinds,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct FilteredIssues<T: RefTrait>(pub BTreeSet<IssueKey<T>>);

// TODO(luca) It seems just deriving default results in compile errors
impl<T: RefTrait> Default for FilteredIssues<T> {
    fn default() -> Self {
        Self(BTreeSet::default())
    }
}

impl<T: RefTrait> FilteredIssues<T> {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<FilteredIssues<U>, T> {
        let mut issues = BTreeSet::new();
        for issue in self.0.iter() {
            let entities = issue
                .entities
                .iter()
                .map(|e| id_map.get(e).cloned().ok_or(*e))
                .collect::<Result<BTreeSet<_>, _>>()?;
            issues.insert(IssueKey {
                entities,
                kind: issue.kind.clone(),
            });
        }
        Ok(FilteredIssues(issues))
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default, PartialEq)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct FilteredIssueKinds(pub BTreeSet<Uuid>);

impl FilteredIssueKinds {
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }
}

impl<T: RefTrait> Default for SiteProperties<T> {
    fn default() -> Self {
        Self {
            name: NameOfSite("new_site".to_owned()),
            geographic_offset: GeographicComponent::default(),
            filtered_issues: FilteredIssues::default(),
            filtered_issue_kinds: FilteredIssueKinds::default(),
        }
    }
}

impl<T: RefTrait> SiteProperties<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<SiteProperties<U>, T> {
        Ok(SiteProperties {
            name: self.name.clone(),
            geographic_offset: self.geographic_offset.clone(),
            filtered_issues: self.filtered_issues.convert(id_map)?,
            filtered_issue_kinds: self.filtered_issue_kinds.clone(),
        })
    }
}

#[derive(Serialize, Deserialize, Debug, Clone, Default)]
pub struct Site {
    /// The site data format that is being used
    pub format_version: SemVer,
    /// Anchors that are relevant across all levels
    // TODO(MXG): Should we use a different name for this to distinguish it
    // from level anchors, or does the grouping make the intent obvious enough?
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub anchors: BTreeMap<u32, Anchor>,
    /// Properties that are tied to the whole site
    pub properties: SiteProperties<u32>,
    /// Properties of each level
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub levels: BTreeMap<u32, Level>,
    /// The groups of textures being used in the site
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub textures: BTreeMap<u32, TextureGroup>,
    /// The fiducial groups that exist in the site
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fiducial_groups: BTreeMap<u32, FiducialGroup>,
    /// The fiducial instances that exist in Cartesian space
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub fiducials: BTreeMap<u32, Fiducial<u32>>,
    /// Properties of each lift
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub lifts: BTreeMap<u32, Lift<u32>>,
    /// Data related to navigation
    #[serde(default, skip_serializing_if = "Navigation::is_empty")]
    pub navigation: Navigation,

    /// Scenarios that exist in the site
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub scenarios: BTreeMap<u32, Scenario<u32>>,
    /// Model descriptions available in this site
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub model_descriptions: BTreeMap<u32, ModelDescriptionBundle>,
    /// Robots available in this site
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub robots: BTreeMap<u32, Robot>,
    /// Model instances that exist in the site
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub model_instances: BTreeMap<u32, Parented<u32, ModelInstance<u32>>>,
    /// Tasks available in this site
    #[serde(default, skip_serializing_if = "BTreeMap::is_empty")]
    pub tasks: BTreeMap<u32, Task>,
}

#[derive(Serialize, Deserialize, Debug, Clone, PartialEq, Eq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component, Deref, DerefMut))]
pub struct NameOfSite(pub String);

fn default_style_config() -> Style {
    Style::new()
        .depth_limit(4)
        .new_line("\n".to_string())
        .indentor("  ".to_string())
        .struct_names(false)
}

impl Site {
    pub fn to_writer_ron<W: io::Write>(&self, mut writer: W) -> ron::Result<()> {
        let mut contents = String::new();
        ron::ser::to_writer_pretty(&mut contents, self, default_style_config())?;
        writer
            .write_all(contents.as_bytes())
            .map_err(ron::Error::from)
    }

    pub fn to_writer_custom_ron<W: io::Write>(
        &self,
        mut writer: W,
        style: Style,
    ) -> ron::Result<()> {
        let mut contents = String::new();
        ron::ser::to_writer_pretty(&mut contents, self, style)?;
        writer
            .write_all(contents.as_bytes())
            .map_err(ron::Error::from)
    }

    pub fn to_string_ron(&self) -> ron::Result<String> {
        ron::ser::to_string_pretty(self, default_style_config())
    }

    pub fn to_string_custom_ron(&self, style: Style) -> ron::Result<String> {
        ron::ser::to_string_pretty(self, style)
    }

    pub fn to_writer_json<W: io::Write>(&self, writer: W) -> serde_json::Result<()> {
        serde_json::to_writer_pretty(writer, self)
    }

    pub fn from_bytes_json(s: &[u8]) -> serde_json::Result<Self> {
        serde_json::from_slice(s)
    }

    pub fn from_reader_ron<R: io::Read>(reader: R) -> ron::error::SpannedResult<Self> {
        // TODO(MXG): Validate the parsed data, e.g. make sure anchor pairs
        // belong to the same level.
        ron::de::from_reader(reader)
    }

    pub fn from_str_ron<'a>(s: &'a str) -> ron::error::SpannedResult<Self> {
        ron::de::from_str(s)
    }

    pub fn to_bytes_json(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec_pretty(self)
    }

    pub fn to_bytes_json_pretty(&self) -> serde_json::Result<Vec<u8>> {
        serde_json::to_vec(self)
    }

    pub fn to_string_json(&self) -> serde_json::Result<String> {
        serde_json::to_string(self)
    }

    pub fn to_string_json_pretty(&self) -> serde_json::Result<String> {
        serde_json::to_string_pretty(self)
    }

    pub fn from_bytes_ron<'a>(s: &'a [u8]) -> ron::error::SpannedResult<Self> {
        ron::de::from_bytes(s)
    }

    /// Returns an anchor and its level (if it's a level anchor), given the id
    pub fn get_anchor_and_level(&self, id: u32) -> Option<(&Anchor, Option<&Level>)> {
        self.anchors
            .get(&id)
            .map(|site_anchor| (site_anchor, None))
            .or_else(|| {
                self.levels.values().find_map(|l| {
                    l.anchors
                        .get(&id)
                        .map(|level_anchor| (level_anchor, Some(l)))
                })
            })
    }

    /// Returns an anchor given the id
    pub fn get_anchor(&self, id: u32) -> Option<&Anchor> {
        self.get_anchor_and_level(id).map(|(a, _)| a)
    }

    #[allow(non_snake_case)]
    pub fn blank_L1(name: String) -> Self {
        let mut site = Site::default();
        site.properties.name = NameOfSite(name);
        site.levels.insert(
            1,
            Level::new(
                LevelProperties {
                    name: NameInSite("L1".to_owned()),
                    elevation: LevelElevation(0.0),
                    ..Default::default()
                },
                RankingsInLevel::default(),
            ),
        );
        site
    }
}

pub trait RefTrait: Ord + Eq + Copy + Send + Sync + Hash + 'static {}

impl RefTrait for u32 {}

#[cfg(feature = "bevy")]
impl RefTrait for Entity {}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::building_map::BuildingMap;

    #[test]
    fn ron_roundtrip() {
        let data = std::fs::read("../../assets/demo_maps/office.building.yaml").unwrap();
        let map = BuildingMap::from_bytes(&data).unwrap();
        let site_string = map.to_site().unwrap().to_string_ron().unwrap();
        println!("{site_string}");
        Site::from_str_ron(&site_string).unwrap();
    }

    #[test]
    fn json_roundtrip() {
        let data = std::fs::read("../../assets/demo_maps/office.building.yaml").unwrap();
        let map = BuildingMap::from_bytes(&data).unwrap();
        let site_string = map.to_site().unwrap().to_bytes_json().unwrap();
        Site::from_bytes_json(&site_string).unwrap();
    }

    #[test]
    fn produce_json_string() {
        let data = std::fs::read("../../assets/demo_maps/office.building.yaml").unwrap();
        let map = BuildingMap::from_bytes(&data).unwrap();
        let text = map.to_site().unwrap().to_string_json_pretty().unwrap();
        println!("{text}");
    }
}
