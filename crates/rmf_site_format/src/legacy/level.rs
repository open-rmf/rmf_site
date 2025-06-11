use super::{
    super::alignment::Alignment, super::Light, door::Door, fiducial::Fiducial, floor::Floor,
    lane::Lane, measurement::Measurement, model::Model, physical_camera::PhysicalCamera,
    vertex::Vertex, wall::Wall,
};
use glam::DVec2;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct LevelDrawing {
    pub filename: String,
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
    pub features: Vec<Feature>,
    pub visible: bool,
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Feature {
    pub id: String,
    pub name: String,
    pub x: f64,
    pub y: f64,
}

impl Feature {
    pub fn to_vec(&self) -> DVec2 {
        DVec2::new(self.x, self.y)
    }
}

#[derive(Deserialize, Serialize, Clone, Debug)]
pub struct Constraint {
    pub ids: [String; 2],
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
    pub features: Vec<Feature>,
    #[serde(default)]
    pub constraints: Vec<Constraint>,
    #[serde(default)]
    pub physical_cameras: Vec<PhysicalCamera>,
    #[serde(default)]
    pub fiducials: Vec<Fiducial>,
    #[serde(default)]
    pub lights: Vec<Light>,
    #[serde(skip)]
    pub alignment: Option<Alignment>,
}
