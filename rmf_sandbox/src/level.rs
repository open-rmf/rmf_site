use crate::lane::Lane;
use crate::measurement::Measurement;
use crate::model::Model;
use crate::vertex::Vertex;
use crate::wall::Wall;
use crate::floor::Floor;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct LevelDrawing {
    filename: String,
}

#[derive(Deserialize, Serialize, Component, Clone, Default)]
pub struct Level {
    pub vertices: Vec<Vertex>,
    pub lanes: Vec<Lane>,
    pub measurements: Vec<Measurement>,
    pub models: Vec<Model>,
    pub walls: Vec<Wall>,
    pub drawing: LevelDrawing,
    pub elevation: f64,
    pub flattened_x_offset: f64,
    pub flattened_y_offset: f64,
    pub floors: Vec<Floor>,
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
