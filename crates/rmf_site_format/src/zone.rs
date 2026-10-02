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
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// A named zone polygon on a level. The boundary closes implicitly.
#[derive(Serialize, Deserialize, Debug, Clone, PartialEq)]
pub struct Zone<T: RefTrait> {
    pub anchors: Path<T>,
    pub name: NameInSite,
}

impl<T: RefTrait> Zone<T> {
    pub fn convert<U: RefTrait>(&self, id_map: &HashMap<T, U>) -> Result<Zone<U>, T> {
        Ok(Zone {
            anchors: self.anchors.convert(id_map)?,
            name: self.name.clone(),
        })
    }
}

impl<T: RefTrait> From<Path<T>> for Zone<T> {
    fn from(anchors: Path<T>) -> Self {
        Self {
            anchors,
            name: NameInSite("<Unnamed>".to_owned()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{Anchor, Level, LevelElevation, Site};

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
