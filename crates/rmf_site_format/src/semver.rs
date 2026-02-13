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

use crate::{CURRENT_MAJOR_VERSION, CURRENT_MINOR_VERSION};
use serde::{Deserialize, Serialize, de::Visitor};

/// rmf_site_format uses a kind of semantic versioning.
/// We will continue to parse every format version starting from 1.0 in
/// perpetuity.
///
/// When a minor version is increased, that means some new optional data fields
/// have been added which can be safely ignored by older versions of rmf_site_format
/// that still have the same major version number. The older version will not be
/// able to benefit from the new features represented by these new data fields,
/// but will be able to fall back on the older behaviors. This forward compatibility
/// should be used with caution because editing and replacing the site file with the
/// older version will erase that data. TODO(Grey): Store unknown fields separately
/// and then naively re-insert them when saving a file.
///
/// When a major version is increased, that means some mandatory expectation of
/// the parser has changed and older versions of the parser can no longer read
/// the new data.
#[derive(Clone, Copy, Debug)]
pub struct SemVer(pub u32, pub u32);

impl SemVer {
    pub fn major(&self) -> u32 {
        self.0
    }

    pub fn minor(&self) -> u32 {
        self.1
    }

    pub fn to_string(&self) -> String {
        format!("{}.{}", self.0, self.1)
    }
}

impl Default for SemVer {
    fn default() -> Self {
        SemVer(CURRENT_MAJOR_VERSION, CURRENT_MINOR_VERSION)
    }
}

impl Serialize for SemVer {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&format!("{}.{}", self.0, self.1))
    }
}

impl<'de> Deserialize<'de> for SemVer {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(SemVerVisitor)
    }
}

struct SemVerVisitor;
impl<'de> Visitor<'de> for SemVerVisitor {
    type Value = SemVer;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str(
            "a string of the form \"MAJOR.MINOR\"  where MAJOR and MINOR are non-negative integers",
        )
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        let split_results: Vec<_> = v.split(".").map(|s| s.parse::<u32>()).collect();
        let mut version_components: [u32; 2] = [0, 0];
        for (i, result) in split_results.iter().enumerate() {
            match result {
                Ok(value) => {
                    if i < 2 {
                        version_components[i] = *value;
                    }
                }
                Err(err) => {
                    return Err(E::custom(err.to_string()));
                }
            }
        }

        if split_results.len() > 2 {
            return Err(E::custom(format!(
                "too many components in format version [{}]; found [{}], but it must be exactly 2",
                v,
                split_results.len(),
            )));
        }

        if split_results.len() < 2 {
            return Err(E::custom(format!(
                "not enough components in format version [{}]; found [{}], but it must be exactly 2",
                v,
                split_results.len(),
            )));
        }

        if version_components[0] > CURRENT_MAJOR_VERSION {
            return Err(E::custom(format!(
                "major version of input data is [{}], but your version of rmf_site_format only supports up to [{}.{}]; try updating to the latest version of rmf_site_format to read this file",
                version_components[0], CURRENT_MAJOR_VERSION, CURRENT_MINOR_VERSION,
            )));
        }

        return Ok(SemVer(version_components[0], version_components[1]));
    }
}
