/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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

use crate::{NameInSite, Path, RefTrait};
#[cfg(feature = "bevy")]
use bevy::prelude::Component;
use serde::{Deserialize, Serialize};
use std::collections::{BTreeSet, HashMap};

/// A named zone polygon on a level. The boundary closes implicitly.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Zone<T: RefTrait> {
    pub anchors: Path<T>,
    pub name: NameInSite,
    #[serde(default, skip_serializing_if = "ZoneSets::is_empty")]
    pub sets: ZoneSets<T>,
}

impl<T: RefTrait> Zone<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Zone<U>, T> {
        Ok(Zone {
            anchors: self.anchors.convert(id_map)?,
            name: self.name.clone(),
            sets: self.sets.convert(id_map)?,
        })
    }
}

impl<T: RefTrait> From<Path<T>> for Zone<T> {
    fn from(anchors: Path<T>) -> Self {
        Self {
            anchors,
            name: NameInSite("<Unnamed>".to_owned()),
            sets: ZoneSets::default(),
        }
    }
}

/// A named set of zones, shared across levels.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct ZoneSet {
    pub name: NameInSite,
}

/// The sets that a zone belongs to.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
#[serde(transparent)]
#[cfg_attr(feature = "bevy", derive(Component))]
pub struct ZoneSets<T: RefTrait>(pub BTreeSet<T>);

impl<T: RefTrait> Default for ZoneSets<T> {
    fn default() -> Self {
        Self(BTreeSet::new())
    }
}

impl<T: RefTrait> ZoneSets<T> {
    fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<ZoneSets<U>, T> {
        self.0
            .iter()
            .map(|id| id_map.get(id).copied().ok_or(*id))
            .collect::<Result<BTreeSet<_>, _>>()
            .map(ZoneSets)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Anchor, Level, LevelElevation, Site};

    #[test]
    fn zone_sets_roundtrip_and_remap_membership() {
        let zone = Zone {
            anchors: Path(vec![1_u32, 2, 3]),
            name: NameInSite("Desk aisle".into()),
            sets: ZoneSets(BTreeSet::from([10, 20])),
        };
        let mapping = HashMap::from([(1, 101_u32), (2, 102), (3, 103), (10, 110), (20, 120)]);
        let converted = zone.convert(&mapping).unwrap();
        assert_eq!(converted.sets.0, BTreeSet::from([110, 120]));
        assert_eq!(zone.sets.convert(&HashMap::from([(10, 110_u32)])), Err(20));
        let mut site = Site::default();
        site.zone_sets.insert(
            10,
            ZoneSet {
                name: NameInSite("Wi-Fi survey".into()),
            },
        );
        site.zone_sets.insert(
            20,
            ZoneSet {
                name: NameInSite("Inspection areas".into()),
            },
        );
        for (level_id, offset) in [(5, 0), (6, 100)] {
            let mut level = Level::default();
            let mapping = HashMap::from([
                (1, offset + 1),
                (2, offset + 2),
                (3, offset + 3),
                (10, 10),
                (20, 20),
            ]);
            for (id, point) in [(1, [0.0, 0.0]), (2, [2.0, 0.0]), (3, [0.0, 2.0])] {
                level.anchors.insert(offset + id, Anchor::from(point));
            }
            level
                .zones
                .insert(offset + 4, zone.convert(&mapping).unwrap());
            site.levels.insert(level_id, level);
        }
        let encoded = serde_json::to_vec(&site).unwrap();
        let decoded = Site::from_bytes_json(&encoded).unwrap();
        assert_eq!(decoded.zone_sets, site.zone_sets);
        for level in decoded.levels.values() {
            assert_eq!(
                level.zones.values().next().unwrap().sets.0,
                BTreeSet::from([10, 20])
            );
        }
        let old: Zone<u32> =
            serde_json::from_value(serde_json::json!({"anchors": [1, 2, 3], "name": "Old zone"}))
                .unwrap();
        assert!(old.sets.is_empty());
        assert!(serde_json::to_value(old).unwrap().get("sets").is_none());
        assert!(
            serde_json::to_value(Site::default())
                .unwrap()
                .get("zone_sets")
                .is_none()
        );
    }

    #[test]
    fn zone_example_has_resolvable_level_boundaries() {
        let site =
            Site::from_bytes_json(include_bytes!("../../../assets/demo_maps/office.site.json"))
                .unwrap();
        assert_eq!(site.levels.len(), 1);
        for level in site.levels.values() {
            assert_eq!(level.zones.len(), 3);
            for zone in level.zones.values() {
                assert!(zone.anchors.0.len() >= 3);
                assert!(
                    zone.anchors
                        .0
                        .iter()
                        .all(|id| level.anchors.contains_key(id))
                );
            }
        }
    }

    #[test]
    fn old_level_without_zones_remains_unchanged() {
        let original = serde_json::to_value(Level::default()).unwrap();
        assert!(original.get("zones").is_none());
        let level: Level = serde_json::from_value(original.clone()).unwrap();
        assert!(level.zones.is_empty());
        assert_eq!(serde_json::to_value(level).unwrap(), original);
    }

    #[test]
    fn zones_roundtrip_across_levels_and_convert_references() {
        let zone = Zone {
            anchors: Path(vec![1_u32, 2, 3]),
            name: NameInSite("Poor Wi-Fi corridor".into()),
            sets: ZoneSets::default(),
        };
        let converted = zone
            .convert(&HashMap::from([(1, 11_u32), (2, 12), (3, 13)]))
            .unwrap();
        assert_eq!(converted.anchors.0, vec![11, 12, 13]);
        assert_eq!(converted.name, zone.name);
        assert_eq!(zone.convert(&HashMap::from([(1, 11_u32)])), Err(2));

        let mut site = Site::default();
        for (level_id, offset, elevation) in [(10, 0, 0.0), (20, 100, 4.5)] {
            let mut level = Level::default();
            level.properties.elevation = LevelElevation(elevation);
            let mapping = HashMap::from([(1, offset + 1), (2, offset + 2), (3, offset + 3)]);
            for (id, point) in [(1, [0.0, 0.0]), (2, [2.0, 0.0]), (3, [0.0, 2.0])] {
                level.anchors.insert(offset + id, Anchor::from(point));
            }
            level
                .zones
                .insert(offset + 4, zone.convert(&mapping).unwrap());
            let mut second = zone.convert(&mapping).unwrap();
            second.name = NameInSite("Second zone sharing anchors".into());
            second.anchors.0.reverse();
            level.zones.insert(offset + 5, second);
            site.levels.insert(level_id, level);
        }
        let encoded = serde_json::to_vec(&site).unwrap();
        let decoded = Site::from_bytes_json(&encoded).unwrap();
        for (id, level) in &site.levels {
            assert_eq!(decoded.levels[id].zones, level.zones);
            assert_eq!(
                decoded.levels[id].properties.elevation,
                level.properties.elevation
            );
            assert_eq!(
                serde_json::to_value(&decoded.levels[id].anchors).unwrap(),
                serde_json::to_value(&level.anchors).unwrap(),
            );
        }
    }
}
