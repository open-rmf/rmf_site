use super::{
    door::Door, fiducial::Fiducial, floor::Floor, lane::Lane, measurement::Measurement,
    model::Model, physical_camera::PhysicalCamera, vertex::Vertex, wall::Wall,
    super::Light,
};
use glam::{DAffine2, DVec2};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct LevelDrawing {
    pub filename: String,
}

#[derive(Clone, Copy)]
pub struct Alignment {
    pub translation: DVec2,
    pub rotation: f64,
    pub scale: f64,
}

impl Alignment {
    pub fn to_affine(&self) -> DAffine2 {
        DAffine2::from_scale_angle_translation(
            DVec2::splat(self.scale),
            self.rotation,
            self.translation,
        )
    }
}

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct Level {
    #[serde(default)]
    pub vertices: Vec<Vertex>,
    #[serde(default)]
    pub lanes: Vec<Lane>,
    #[serde(default)]
    pub measurements: Vec<Measurement>,
    #[serde(default)]
    pub models: Vec<Model>,
    #[serde(default)]
    pub walls: Vec<Wall>,
    #[serde(default)]
    pub doors: Vec<Door>,
    #[serde(default)]
    pub drawing: LevelDrawing,
    pub elevation: f64,
    #[serde(default)]
    pub floors: Vec<Floor>,
    #[serde(default)]
    pub physical_cameras: Vec<PhysicalCamera>,
    #[serde(default)]
    pub fiducials: Vec<Fiducial>,
    #[serde(default)]
    pub lights: Vec<Light>,
    #[serde(skip)]
    pub alignment: Option<Alignment>,
}
