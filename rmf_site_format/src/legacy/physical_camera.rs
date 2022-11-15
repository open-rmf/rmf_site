use crate::{
    Angle, NameInSite, PhysicalCamera as SitePhysicalCamera, PhysicalCameraProperties, Pose,
    Rotation,
};
use glam::DVec2;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct PhysicalCamera {
    // extrinsic properties
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub pitch: f64,
    pub yaw: f64,
    // intrinsic properties
    pub image_fov: f64,
    pub image_width: u32,
    pub image_height: u32,
    pub update_rate: u32,
}

impl PhysicalCamera {
    pub fn to_vec(&self) -> DVec2 {
        DVec2::new(self.x, self.y)
    }

    pub fn to_site(&self) -> SitePhysicalCamera {
        SitePhysicalCamera {
            name: NameInSite(self.name.clone()),
            pose: Pose {
                trans: [self.x as f32, self.y as f32, self.z as f32],
                rot: Rotation::EulerExtrinsicXYZ([
                    Angle::Deg(0.),
                    Angle::Deg(self.pitch.to_degrees() as f32),
                    Angle::Deg(self.yaw.to_degrees() as f32),
                ]),
            },
            properties: PhysicalCameraProperties {
                width: self.image_width,
                height: self.image_height,
                horizontal_fov: self.image_fov as f32,
                frame_rate: self.update_rate as f32,
            },
        }
    }
}
