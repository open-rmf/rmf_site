use super::{rbmf::*, PortingError, Result};
use crate::{AssetSource, Texture, Wall as SiteWall, DEFAULT_LEVEL_HEIGHT};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

fn default_height() -> RbmfFloat {
    RbmfFloat::from(DEFAULT_LEVEL_HEIGHT as f64)
}

fn default_width() -> RbmfFloat {
    RbmfFloat::from(1.0)
}

fn default_scale() -> RbmfFloat {
    RbmfFloat::from(1.0)
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WallProperties {
    pub alpha: RbmfFloat,
    pub texture_name: RbmfString,
    #[serde(default = "default_height")]
    pub texture_height: RbmfFloat,
    #[serde(default = "default_width")]
    pub texture_width: RbmfFloat,
    #[serde(default = "default_scale")]
    pub texture_scale: RbmfFloat,
}

impl Default for WallProperties {
    fn default() -> Self {
        Self {
            alpha: RbmfFloat::default(),
            texture_name: RbmfString::from("default".to_string()),
            texture_height: default_height(),
            texture_width: default_width(),
            texture_scale: default_scale(),
        }
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Wall(pub usize, pub usize, pub WallProperties);

impl Wall {
    pub fn to_site(&self, vertex_to_anchor_id: &HashMap<usize, u32>) -> Result<SiteWall<u32>> {
        let left_anchor = vertex_to_anchor_id
            .get(&self.0)
            .ok_or(PortingError::InvalidVertex(self.0))?;
        let right_anchor = vertex_to_anchor_id
            .get(&self.1)
            .ok_or(PortingError::InvalidVertex(self.1))?;
        Ok(SiteWall {
            anchors: [*left_anchor, *right_anchor].into(),
            texture: if self.2.texture_name.is_empty() {
                Texture {
                    source: AssetSource::Remote(
                        "Luca/RMF_Materials/textures/default.png".to_owned(),
                    ),
                    ..Default::default()
                }
            } else {
                Texture {
                    source: AssetSource::Remote(
                        "Luca/RMF_Materials/textures/".to_owned() + &self.2.texture_name.1 + ".png",
                    ),
                    alpha: Some(self.2.alpha.1 as f32),
                    width: Some((self.2.texture_width.1 / self.2.texture_scale.1) as f32),
                    height: Some((self.2.texture_height.1 / self.2.texture_scale.1) as f32),
                    ..Default::default()
                }
            },
            marker: Default::default(),
        })
    }
}
