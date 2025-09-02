use strum::EnumCount;

pub const Z_MIN: f32 = 0.0;
pub const Z_MAX: f32 = 0.01;

/// Ordered based on lowest z height
#[derive(Debug, Clone, Copy, EnumCount)]
pub enum ZLayer {
    Drawing = 0,
    Floor,
    Measurement,
    Lane,
    Doormat,
    OccupancyGrid,
    RobotPath,
    Location,
    SelectedLane,
    HoveredLane,
    LabelText,
}

impl ZLayer {
    pub fn to_z(&self) -> f32 {
        // Turn enum value to usize as priority
        let priority = *self as usize;
        // Assumes constant offset layer-to-layer
        let offset = (Z_MAX - Z_MIN) / ((Self::COUNT - 1) as f32);
        return Z_MIN + ((priority as f32) * offset);
    }
}
