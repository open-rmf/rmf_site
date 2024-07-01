use crate::{
    Affiliation, Angle, AssetSource, Group, IsStatic, Model as SiteModel, ModelDescription,
    ModelInstance, ModelMarker, NameInSite, SiteParentID, Pose, Rotation, Scale,
};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::{
    collections::{BTreeMap, HashMap},
    ops::RangeFrom,
};

#[derive(Deserialize, Serialize, Clone, Default, Debug)]
pub struct Model {
    pub model_name: String,
    #[serde(rename = "name")]
    pub instance_name: String,
    #[serde(rename = "static")]
    pub static_: bool,
    pub x: f64,
    pub y: f64,
    #[serde(rename = "z")]
    pub z_offset: f64,
    pub yaw: f64,
}

impl Model {
    pub fn to_vec(&self) -> DVec2 {
        DVec2::new(self.x, self.y)
    }

    pub fn to_site(&self) -> SiteModel {
        SiteModel {
            name: NameInSite(self.instance_name.clone()),
            source: AssetSource::Search(self.model_name.clone()),
            pose: Pose {
                trans: [self.x as f32, self.y as f32, self.z_offset as f32],
                rot: Rotation::Yaw(Angle::Deg(self.yaw.to_degrees() as f32)),
            },
            is_static: IsStatic(self.static_),
            scale: Scale::default(),
            marker: ModelMarker,
        }
    }

    pub fn to_model_instance(
        &self,
        model_description_name_map: &mut HashMap<String, u32>,
        model_descriptions: &mut BTreeMap<u32, ModelDescription>,
        site_id: &mut RangeFrom<u32>,
        level_id: u32,
    ) -> ModelInstance<u32> {
        let model_description_id = match model_description_name_map.get(&self.model_name) {
            Some(id) => *id,
            None => {
                let id = site_id.next().unwrap();
                model_description_name_map.insert(self.model_name.clone(), id);
                model_descriptions.insert(
                    id,
                    ModelDescription {
                        name: NameInSite(self.model_name.clone()),
                        source: AssetSource::Search(self.model_name.clone()),
                        group: Group::default(),
                        marker: ModelMarker,
                    },
                );
                id
            }
        };

        ModelInstance {
            name: NameInSite(self.instance_name.clone()),
            source: AssetSource::Search(self.model_name.clone()),
            pose: Pose {
                trans: [self.x as f32, self.y as f32, self.z_offset as f32],
                rot: Rotation::Yaw(Angle::Deg(self.yaw.to_degrees() as f32)),
            },
            parent: SiteParentID(level_id),
            description: Affiliation(Some(model_description_id)),
            marker: ModelMarker,
        }
    }
}
