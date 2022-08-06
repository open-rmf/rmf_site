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
