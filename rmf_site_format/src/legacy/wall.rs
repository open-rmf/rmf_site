use super::{rbmf::*, PortingError, Result};
use crate::{AssetSource, Texture, Wall as SiteWall};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

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
                    offset: Some((0., self.2.texture_height.1 as f32)),
                    ..Default::default()
                }
            },
            marker: Default::default(),
        })
    }
}
