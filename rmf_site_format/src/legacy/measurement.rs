use super::rbmf::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct MeasurementProperties {
    // TODO: For new cartesian format, there should be no need for this value since the
    // metric distance is always equal to the distance between start and end.
    pub distance: RbmfFloat,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Measurement(pub usize, pub usize, pub MeasurementProperties);
