use super::{rbmf::*, PortingError, Result};
use crate::{Angle, CustomTexture, Floor as SiteFloor, FloorMarker, Path, Texture, TextureSource, PreferredSemiTransparency};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct FloorParameters {
    pub texture_name: RbmfString,
    pub texture_rotation: RbmfFloat,
    pub texture_scale: RbmfFloat,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Floor {
    pub parameters: FloorParameters,
    pub vertices: Vec<usize>,
}

impl Floor {
    pub fn to_site(&self, vertex_to_anchor_id: &HashMap<usize, u32>) -> Result<SiteFloor<u32>> {
        let mut anchors = Vec::new();
        for v in &self.vertices {
            let anchor = *vertex_to_anchor_id
                .get(v)
                .ok_or(PortingError::InvalidVertex(*v))?;

            anchors.push(anchor);
        }

        Ok(SiteFloor {
            anchors: Path(anchors),
            texture: if self.parameters.texture_name.1.is_empty() {
                Texture::Default
            } else {
                Texture::Custom(CustomTexture {
                    source: TextureSource::Filename(self.parameters.texture_name.1.clone()),
                    alpha: None,
                    rotation: Some(Angle::Deg(self.parameters.texture_rotation.1 as f32)),
                    scale: Some(self.parameters.texture_scale.1 as f32),
                    offset: None,
                })
            },
            preferred_semi_transparency: PreferredSemiTransparency::for_floor(),
            marker: FloorMarker,
        })
    }
}
