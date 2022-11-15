use glam::DVec2;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Fiducial(pub f64, pub f64, pub String);

impl Fiducial {
    pub fn to_vec(&self) -> DVec2 {
        DVec2::new(self.0, self.1)
    }
}
