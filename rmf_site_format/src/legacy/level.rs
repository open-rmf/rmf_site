use super::{
    door::Door, fiducial::Fiducial, floor::Floor, lane::Lane, measurement::Measurement,
    model::Model, physical_camera::PhysicalCamera, vertex::Vertex, wall::Wall,
};
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct LevelDrawing {
    pub filename: String,
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
    pub flattened_x_offset: f64,
    pub flattened_y_offset: f64,
    #[serde(default)]
    pub floors: Vec<Floor>,
    #[serde(default)]
    pub physical_cameras: Vec<PhysicalCamera>,
    #[serde(default)]
    pub fiducials: Vec<Fiducial>,
}

impl Level {
    pub fn calc_bb(&self) -> BoundingBox2D {
        let mut bb = BoundingBox2D {
            min_x: 1e100,
            max_x: -1e100,
            min_y: 1e100,
            max_y: -1e100,
        };
        for v in self.vertices.iter() {
            if v.0 < bb.min_x {
                bb.min_x = v.0;
            }
            if v.0 > bb.max_x {
                bb.max_x = v.0;
            }
            if v.1 < bb.min_y {
                bb.min_y = v.1;
            }
            if v.1 > bb.max_y {
                bb.max_y = v.1;
            }
        }
        bb
    }
}

pub struct BoundingBox2D {
    pub min_x: f64,
    pub max_x: f64,
    pub min_y: f64,
    pub max_y: f64,
}
