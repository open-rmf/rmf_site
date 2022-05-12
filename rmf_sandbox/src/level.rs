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

        // todo: calculate scale and inter-level alignment
        let mut ofs_x = 0.0;
        let mut ofs_y = 0.0;
        let mut num_v = 0;
        for v in &level.vertices {
            ofs_x += v.x_raw;
            ofs_y += v.y_raw;
            num_v += 1;
        }
        ofs_x /= num_v as f64;
        ofs_y /= num_v as f64;

        let mut n_dist = 0;
        let mut sum_dist = 0.;
        for meas in &level.measurements {
            let dx_raw = level.vertices[meas.start].x_raw - level.vertices[meas.end].x_raw;
            let dy_raw = level.vertices[meas.start].y_raw - level.vertices[meas.end].y_raw;
            let dist_raw = (dx_raw * dx_raw + dy_raw * dy_raw).sqrt();
            let dist_meters = meas.distance;
            sum_dist += dist_meters / dist_raw;
            n_dist += 1;
        }
        let scale = match n_dist {
            0 => 1.0,
            _ => sum_dist / n_dist as f64,
        };
        println!("scale: {}", scale);

        for v in level.vertices.iter_mut() {
            v.x_meters = (v.x_raw - ofs_x) * scale;
            v.y_meters = (v.y_raw - ofs_y) * scale;
        }

        for m in level.models.iter_mut() {
            m.x_meters = (m.x_raw - ofs_x) * scale;
            m.y_meters = (m.y_raw - ofs_y) * scale;
        }
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
