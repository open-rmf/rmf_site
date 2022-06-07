use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Component)]
pub struct Fiducial(f64, f64, String);
