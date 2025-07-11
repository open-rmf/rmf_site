use serde::{Deserialize, Serialize};

#[cfg(feature = "bevy")]
use bevy::prelude::*;

/// Geographic Offset for the
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[cfg_attr(feature = "bevy", derive(Reflect))]
pub struct GeographicOffset {
    /// Contains the latitude and longitude pair for
    pub anchor: (f32, f32),

    /// Zoom level
    pub zoom: i32,

    /// Visibility of the map
    pub visible: bool,
}

#[cfg_attr(feature = "bevy", derive(Component, Reflect))]
#[cfg_attr(feature = "bevy", reflect(Component))]
#[derive(Serialize, Deserialize, Debug, Clone, Copy, Default)]
#[serde(transparent)]
pub struct GeographicComponent(pub Option<GeographicOffset>);

impl GeographicComponent {
    pub fn is_none(&self) -> bool {
        self.0.is_none()
    }
}

impl GeographicOffset {
    pub fn from_latlon(latlon: (f32, f32)) -> Self {
        Self {
            anchor: latlon.clone(),
            zoom: 15,
            ..Default::default()
        }
    }
}
