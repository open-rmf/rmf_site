use super::{rbmf::*, PortingError, Result};
use crate::{
    Affiliation, Angle, AssetSource, Floor as SiteFloor, FloorMarker, Path,
    PreferredSemiTransparency, Texture,
};
use bevy_ecs::prelude::Entity;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::RangeFrom,
};

#[derive(Deserialize, Serialize, Clone, Default, Hash, PartialEq, Eq)]
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
    pub fn to_site(
        &self,
        vertex_to_anchor_id: &HashMap<usize, Entity>,
        textures: &mut BTreeMap<Entity, Texture>,
        texture_map: &mut HashMap<FloorParameters, Entity>,
        site_id: &mut RangeFrom<u32>,
    ) -> Result<SiteFloor> {
        let mut anchors = Vec::new();
        for v in &self.vertices {
            let anchor = *vertex_to_anchor_id
                .get(v)
                .ok_or(PortingError::InvalidVertex(*v))?;

            anchors.push(anchor);
        }

        let texture_site_id = *texture_map
            .entry(self.parameters.clone())
            .or_insert_with(|| {
                let texture = if self.parameters.texture_name.1.is_empty() {
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
                                + &self.parameters.texture_name.1
                                + ".png",
                        ),
                        rotation: Some(Angle::Deg(self.parameters.texture_rotation.1 as f32)),
                        width: Some(self.parameters.texture_scale.1 as f32),
                        height: Some(self.parameters.texture_scale.1 as f32),
                        ..Default::default()
                    }
                };

                let texture_site_id = Entity::from_raw(site_id.next().unwrap());
                textures.insert(texture_site_id, texture);
                texture_site_id
            });

        Ok(SiteFloor {
            anchors: Path(anchors),
            texture: Affiliation(Some(texture_site_id)),
            preferred_semi_transparency: PreferredSemiTransparency::for_floor(),
            marker: FloorMarker,
        })
    }
}
