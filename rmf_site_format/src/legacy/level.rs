use super::{
    super::Light, door::Door, fiducial::Fiducial, floor::Floor, lane::Lane,
    measurement::Measurement, model::Model, physical_camera::PhysicalCamera, vertex::Vertex,
    wall::Wall,
};
use glam::{DAffine2, DVec2};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct LevelDrawing {
    pub filename: String,
}

#[derive(Debug, Clone, Copy)]
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

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct LayerTransform {
    pub scale: f64,
    pub translation_x: f64,
    pub translation_y: f64,
    pub yaw: f64,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Layer {
    // TODO(luca) add color and features
    pub filename: String,
    pub transform: LayerTransform,
    pub visible: bool,
}

// TODO(luca) add layers vector for robot maps
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
    pub layers: BTreeMap<String, Layer>,
    #[serde(default)]
    pub physical_cameras: Vec<PhysicalCamera>,
    #[serde(default)]
    pub fiducials: Vec<Fiducial>,
    #[serde(default)]
    pub lights: Vec<Light>,
    #[serde(skip)]
    pub alignment: Option<Alignment>,
}
