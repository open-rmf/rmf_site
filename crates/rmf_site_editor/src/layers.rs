use strum::EnumCount;

pub const DRAWING_LAYER_START: f32 = 0.0;
pub const MAX_Z_LIMIT: f32 = 0.01;

/// Ordered based on lowest z height
///  
#[derive(Debug, EnumCount)]
#[repr(u8)]
pub enum ZLayer {
    Draw,
    Floor,
    Measurement,
    /// Check z-fighting with recency ranking
    Lane,
    Doormat,
    OccupancyGrid,
    RobotPath,
    Location,
    SelectedLane,
    HoveredLane,
}

impl ZLayer {
    fn into(&self) -> u8 {
        unsafe { *<*const _>::from(self).cast::<u8>() }
    }
    pub fn get_offset() -> f32 {
        // Assumes constant offset layer-to-layer
        return (MAX_Z_LIMIT - DRAWING_LAYER_START) / (Self::COUNT as f32);
    }
    pub fn to_z(&self) -> f32 {
        // Turns enum value to u8, as priority
        let priority: u8 = self.into();
        let offset = Self::get_offset();
        return DRAWING_LAYER_START + ((priority as f32) * offset);
    }
}

// TODO(@mxgrey): Consider using recency rankings for Locations so they don't
// experience z-fighting.
