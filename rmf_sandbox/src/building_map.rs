use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use crate::crowd_sim::CrowdSim;
use crate::level::Level;
use crate::lift::Lift;

#[derive(Deserialize, Serialize, Clone)]
#[serde(rename_all="snake_case")]
pub enum CoordinateSystem {
    ReferenceImage,
    CartesianMeters,
}

impl Default for CoordinateSystem {
    fn default() -> Self {
        CoordinateSystem::ReferenceImage
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct BuildingMap {
    pub name: String,
    #[serde(default)]
    pub coordinate_system: CoordinateSystem,
    pub levels: BTreeMap<String, Level>,
    pub crowd_sim: CrowdSim,
    pub lifts: BTreeMap<String, Lift>,
}

impl BuildingMap {
    pub fn from_bytes(data: &[u8]) -> serde_yaml::Result<BuildingMap> {
        let map: BuildingMap = serde_yaml::from_slice(data)?;
        match map.coordinate_system {
            CoordinateSystem::ReferenceImage => {
                Ok(BuildingMap::from_pixel_coordinates(map))
            }
            CoordinateSystem::CartesianMeters => {
                Ok(map)
            }
        }
    }

    /// Converts a map from the legacy format, which uses pixel coordinates.
    fn from_pixel_coordinates(mut map: BuildingMap) -> BuildingMap {
        for (_, level) in map.levels.iter_mut() {
            // todo: calculate scale and inter-level alignment
            let mut ofs_x = 0.0;
            let mut ofs_y = 0.0;
            let mut num_v = 0;
            for v in &level.vertices {
                ofs_x += v.0;
                ofs_y += -v.1;
                num_v += 1;
            }
            ofs_x /= num_v as f64;
            ofs_y /= num_v as f64;

            // try to guess the scale by averaging the measurement distances.
            let mut n_dist = 0;
            let mut sum_dist = 0.;
            for meas in &level.measurements {
                let dx_raw = level.vertices[meas.0].0 - level.vertices[meas.1].0;
                let dy_raw = level.vertices[meas.0].1 - level.vertices[meas.1].1;
                let dist_raw = (dx_raw * dx_raw + dy_raw * dy_raw).sqrt();
                let dist_meters = *meas.2.distance;
                sum_dist += dist_meters / dist_raw;
                n_dist += 1;
            }
            let scale = match n_dist {
                0 => 1.0,
                _ => sum_dist / n_dist as f64,
            };
            println!("scale: {}", scale);

            // convert to meters
            for v in level.vertices.iter_mut() {
                v.0 = (v.0 - ofs_x) * scale;
                v.1 = (-v.1 - ofs_y) * scale;
            }

            for m in level.models.iter_mut() {
                m.x = (m.x - ofs_x) * scale;
                m.y = (-m.y - ofs_y) * scale;
            }
        }
        map.coordinate_system = CoordinateSystem::CartesianMeters;
        map
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn building_map_serialization() -> Result<(), Box<dyn Error>> {
        let data = std::fs::read("assets/demo_maps/office.building.yaml")?;
        let map = BuildingMap::from_bytes(&data)?;
        std::fs::create_dir_all("test_output")?;
        let out_file = std::fs::File::create("test_output/office.building.yaml")?;
        serde_yaml::to_writer(out_file, &map)?;
        Ok(())
    }
}
