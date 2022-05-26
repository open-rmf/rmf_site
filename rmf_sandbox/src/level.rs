use super::lane::Lane;
use super::level_transform::LevelTransform;
use super::measurement::Measurement;
use super::model::Model;
use super::vertex::Vertex;
use super::wall::Wall;
use bevy::prelude::*;

#[derive(serde::Deserialize, Component, Clone, Default)]
#[serde(from = "LevelRaw")]
pub struct Level {
    pub vertices: Vec<Vertex>,
    pub lanes: Vec<Lane>,
    pub measurements: Vec<Measurement>,
    pub models: Vec<Model>,
    pub walls: Vec<Wall>,
    pub elevation: Option<f64>,
    pub transform: LevelTransform,
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
            if v.x < bb.min_x {
                bb.min_x = v.x;
            }
            if v.x > bb.max_x {
                bb.max_x = v.x;
            }
            if v.y < bb.min_y {
                bb.min_y = v.y;
            }
            if v.y > bb.max_y {
                bb.max_y = v.y;
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

impl From<LevelRaw> for Level {
    fn from(raw: LevelRaw) -> Self {
        let mut level = Level {
            vertices: raw.vertices,
            lanes: raw.lanes,
            measurements: raw.measurements,
            models: raw.models,
            walls: raw.walls,
            ..default()
        };

        level.transform.translation[2] = match raw.elevation {
            Some(e) => e,
            None => 0.,
        };
        level
    }
}

#[derive(serde::Deserialize)]
struct LevelRaw {
    vertices: Vec<Vertex>,
    lanes: Vec<Lane>,
    measurements: Vec<Measurement>,
    models: Vec<Model>,
    walls: Vec<Wall>,
    elevation: Option<f64>,
}
