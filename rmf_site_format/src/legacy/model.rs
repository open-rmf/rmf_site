use crate::{
    Angle, AssetSource, DecorDescription, DecorMarker, Group, IsStatic, MobileRobotDescription, Model as SiteModel, ModelDescriptions, ModelInstance, ModelMarker, NameInSite, Pose, Rotation, Scale
};
use bevy::utils::default;
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, ops::RangeFrom};

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

    pub fn to_decor_instance(
        &self,
        level_id: &u32,
        site_id: &mut RangeFrom<u32>,
        model_description_name_map: &mut HashMap<String, u32>,
        model_descriptions: &mut ModelDescriptions,
    ) -> ModelInstance {
        let pose = Pose {
            trans: [self.x as f32, self.y as f32, self.z_offset as f32],
            rot: Rotation::Yaw(Angle::Deg(self.yaw.to_degrees() as f32)),
        };

        let model_description_id = model_description_name_map
            .entry(self.model_name.clone())
            .or_insert_with(|| {
                let id = site_id.next().unwrap();
                model_descriptions.decors.insert(
                    id,
                    DecorDescription {
                        name: NameInSite(self.model_name.clone()),
                        source: AssetSource::Search(self.model_name.clone()),
                        marker: DecorMarker,
                        group: Group,
                    },
                );
                id
            });

        ModelInstance {
            parent: level_id.clone(),
            model_description: model_description_id.clone(),
            bundle: crate::ModelInstanceBundle {
                name: NameInSite(self.instance_name.clone()),
                pose: Pose {
                    trans: [self.x as f32, self.y as f32, self.z_offset as f32],
                    rot: Rotation::Yaw(Angle::Deg(self.yaw.to_degrees() as f32)),
                },
            },
        }
    }
}
