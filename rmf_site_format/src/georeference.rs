use serde::{Deserialize, Serialize};

/// Geographic Offset for the
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
pub struct GeographicOffset {
    /// Contains the latitude and longitude pair for
    pub anchor: (f32, f32),
}

impl GeographicOffset {
    pub fn from_latlon(latlon: (f32, f32)) -> Self {
        Self {
            anchor: latlon.clone(),
        }
    }
}
