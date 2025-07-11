use super::{rbmf::*, PortingError, Result};
use crate::{Distance, Measurement as SiteMeasurement};
use bevy_ecs::prelude::Entity;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct MeasurementProperties {
    // TODO: For new cartesian format, there should be no need for this value since the
    // metric distance is always equal to the distance between start and end.
    pub distance: RbmfFloat,
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Measurement(pub usize, pub usize, pub MeasurementProperties);

impl Measurement {
    pub fn to_site(&self, vertex_to_anchor_id: &HashMap<usize, Entity>) -> Result<SiteMeasurement> {
        let left_anchor = vertex_to_anchor_id
            .get(&self.0)
            .ok_or(PortingError::InvalidVertex(self.0))?;
        let right_anchor = vertex_to_anchor_id
            .get(&self.1)
            .ok_or(PortingError::InvalidVertex(self.1))?;

        Ok(SiteMeasurement {
            anchors: [*left_anchor, *right_anchor].into(),
            distance: Distance(Some(self.2.distance.1 as f32)),
            marker: Default::default(),
        })
    }
}
