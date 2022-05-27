use std::collections::HashMap;
use serde::{Deserialize, Serialize};

use crate::level::Level;

#[derive(Deserialize, Serialize)]
pub struct BuildingMap {
    pub name: String,
    pub levels: HashMap<String, Level>,
}

impl BuildingMap {
    pub fn from_bytes(data: &[u8]) -> serde_yaml::Result<BuildingMap> {
        // TODO: detect if file is old pixel coordinates or new cartesian format.
        BuildingMap::from_bytes_legacy(data)
    }

    fn from_bytes_cartesian(data: &[u8]) -> serde_yaml::Result<BuildingMap> {
        serde_yaml::from_slice(data)
    }

    /// Loads a legacy building map which uses pixel coordinates.
    fn from_bytes_legacy(data: &[u8]) -> serde_yaml::Result<BuildingMap> {
        let mut map = BuildingMap::from_bytes_cartesian(data)?;
        for (_, level) in map.levels.iter_mut() {
            // todo: calculate scale and inter-level alignment
            let mut ofs_x = 0.0;
            let mut ofs_y = 0.0;
            let mut num_v = 0;
            for v in &level.vertices {
                ofs_x += v.x;
                ofs_y += -v.y;
                num_v += 1;
            }
            ofs_x /= num_v as f64;
            ofs_y /= num_v as f64;

            // try to guess the scale by averaging the measurement distances.
            let mut n_dist = 0;
            let mut sum_dist = 0.;
            for meas in &level.measurements {
                let dx_raw = level.vertices[meas.0].x - level.vertices[meas.1].x;
                let dy_raw = level.vertices[meas.0].y - level.vertices[meas.1].y;
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
                v.x = (v.x - ofs_x) * scale;
                v.y = (-v.y - ofs_y) * scale;
            }

            for m in level.models.iter_mut() {
                m.x = (m.x - ofs_x) * scale;
                m.y = (-m.y - ofs_y) * scale;
            }
        }
        Ok(map)
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
