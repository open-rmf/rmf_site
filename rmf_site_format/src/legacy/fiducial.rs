use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone)]
pub struct Fiducial(pub f64, pub f64, pub String);
