use super::lane::Lane;
use super::level_transform::LevelTransform;
use super::measurement::Measurement;
use super::model::Model;
use super::vertex::Vertex;
use super::wall::Wall;
use bevy::prelude::*;
use serde::{Deserialize, Serialize};

#[derive(Deserialize, Serialize, Clone, Default)]
pub struct LevelDrawing {
    filename: String,
}

#[derive(Deserialize, Serialize, Component, Clone, Default)]
#[serde(from = "LevelRaw", into = "LevelRaw")]
pub struct Level {
    pub vertices: Vec<Vertex>,
    pub lanes: Vec<Lane>,
    pub measurements: Vec<Measurement>,
    pub models: Vec<Model>,
    pub walls: Vec<Wall>,
    pub drawing: LevelDrawing,
    pub flattened_x_offset: f64,
    pub flattened_y_offset: f64,
    #[serde(skip)]
    pub transform: LevelTransform,
}

impl From<LevelRaw> for Level {
    fn from(raw: LevelRaw) -> Self {
        Level {
            vertices: raw.vertices,
            lanes: raw.lanes,
            measurements: raw.measurements,
            models: raw.models,
            walls: raw.walls,
            drawing: raw.drawing,
            flattened_x_offset: raw.flattened_x_offset,
            flattened_y_offset: raw.flattened_y_offset,
            transform: LevelTransform {
                yaw: 0.,
                translation: [0., 0., raw.elevation],
            },
        }
    }
}

impl Into<LevelRaw> for Level {
    fn into(self) -> LevelRaw {
        LevelRaw {
            vertices: self.vertices,
            lanes: self.lanes,
            measurements: self.measurements,
            models: self.models,
            walls: self.walls,
            drawing: self.drawing,
            elevation: self.transform.translation[2],
            flattened_x_offset: self.flattened_x_offset,
            flattened_y_offset: self.flattened_y_offset,
        }
    }
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

#[derive(Deserialize, Serialize)]
struct LevelRaw {
    vertices: Vec<Vertex>,
    lanes: Vec<Lane>,
    measurements: Vec<Measurement>,
    models: Vec<Model>,
    walls: Vec<Wall>,
    drawing: LevelDrawing,
    elevation: f64,
    flattened_x_offset: f64,
    flattened_y_offset: f64,
}
