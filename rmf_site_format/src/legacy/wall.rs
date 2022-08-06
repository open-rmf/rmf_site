use super::rbmf::*;
use serde::{Deserialize, Serialize};

fn default_height() -> RbmfFloat {
    RbmfFloat::from(2.)
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WallProperties {
    pub alpha: RbmfFloat,
    pub texture_name: RbmfString,
    #[serde(default = "default_height")]
    pub texture_height: RbmfFloat,
}

impl Default for WallProperties {
    fn default() -> Self {
        Self {
            alpha: RbmfFloat::default(),
            texture_name: RbmfString::from("default".to_string()),
            texture_height: default_height(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Wall(pub usize, pub usize, pub WallProperties);
