use crate::{Model as SiteModel, Pose, Rotation, Angle, NameInSite, Label, IsStatic, ModelMarker};
use serde::{Deserialize, Serialize};

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
    pub fn to_site(&self) -> SiteModel {
        SiteModel{
            name: NameInSite(self.instance_name.clone()),
            kind: if self.model_name.is_empty() {
                Label(None)
            } else {
                Label(Some(self.model_name.clone()))
            },
            pose: Pose{
                trans: [self.x as f32, self.y as f32, self.z_offset as f32],
                rot: Rotation::Yaw(Angle::Deg(self.yaw.to_degrees() as f32)),
            },
            is_static: IsStatic(self.static_),
            marker: ModelMarker,
        }
    }
}
