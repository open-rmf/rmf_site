use std::collections::HashMap;

use crate::level::Level;

#[derive(serde::Deserialize)]
pub struct BuildingMap {
    pub name: String,
    pub levels: HashMap<String, Level>,
}

impl BuildingMap {
    pub fn from_bytes(data: &[u8]) -> serde_yaml::Result<BuildingMap> {
        serde_yaml::from_slice(data)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::error::Error;

    #[test]
    fn deserialize_building_map() -> Result<(), Box<dyn Error>> {
        let data = std::fs::read("assets/demo_maps/office.building.yaml")?;
        BuildingMap::from_bytes(&data)?;
        Ok(())
    }
}
