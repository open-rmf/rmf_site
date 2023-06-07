use super::{rbmf::*, PortingError, Result};
use crate::{Angle, AssetSource, Floor as SiteFloor, FloorMarker, Path, Texture};
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
                Texture {
                    source: AssetSource::Remote(
                        "Luca/RMF_Materials/textures/blue_linoleum.png".to_owned(),
                    ),
                    ..Default::default()
                }
            } else {
                Texture {
                    source: AssetSource::Remote(
                        "Luca/RMF_Materials/textures/".to_owned()
                            + &self.parameters.texture_name.1
                            + ".png",
                    ),
                    rotation: Some(Angle::Deg(self.parameters.texture_rotation.1 as f32)),
                    scale: Some(self.parameters.texture_scale.1 as f32),
                    ..Default::default()
                }
            },
            marker: FloorMarker,
        })
    }
}
