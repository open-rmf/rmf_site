use super::{PortingError, Result, rbmf::*};
use crate::{Affiliation, AssetSource, DEFAULT_LEVEL_HEIGHT, Texture, Wall as SiteWall};
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::RangeFrom,
};

fn default_height() -> RbmfFloat {
    RbmfFloat::from(DEFAULT_LEVEL_HEIGHT as f64)
}

fn default_width() -> RbmfFloat {
    RbmfFloat::from(1.0)
}

fn default_scale() -> RbmfFloat {
    RbmfFloat::from(1.0)
}

#[derive(Deserialize, Serialize, Clone, Hash, PartialEq, Eq)]
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
    pub fn to_site(
        &self,
        vertex_to_anchor_id: &HashMap<usize, u32>,
        textures: &mut BTreeMap<u32, Texture>,
        texture_map: &mut HashMap<WallProperties, u32>,
        site_id: &mut RangeFrom<u32>,
    ) -> Result<SiteWall<u32>> {
        let left_anchor = vertex_to_anchor_id
            .get(&self.0)
            .ok_or(PortingError::InvalidVertex(self.0))?;
        let right_anchor = vertex_to_anchor_id
            .get(&self.1)
            .ok_or(PortingError::InvalidVertex(self.1))?;

        let texture_site_id = *texture_map.entry(self.2.clone()).or_insert_with(|| {
            let texture = if self.2.texture_name.1.is_empty() {
                Texture {
                    source: AssetSource::Remote(
                        "OpenRobotics/RMF_Materials/textures/blue_linoleum.png".to_owned(),
                    ),
                    ..Default::default()
                }
            } else {
                Texture {
                    source: AssetSource::Remote(
                        "OpenRobotics/RMF_Materials/textures/".to_owned()
                            + &self.2.texture_name.1
                            + ".png",
                    ),
                    rotation: None,
                    width: Some((self.2.texture_width.1 / self.2.texture_scale.1) as f32),
                    height: Some((self.2.texture_height.1 / self.2.texture_scale.1) as f32),
                    alpha: Some(self.2.alpha.1 as f32),
                }
            };

            let texture_site_id = site_id.next().unwrap();
            textures.insert(texture_site_id, texture);
            texture_site_id
        });

        Ok(SiteWall {
            anchors: [*left_anchor, *right_anchor].into(),
            texture: Affiliation(Some(texture_site_id)),
            marker: Default::default(),
        })
    }
}
