use bevy::prelude::*;
use serde_yaml;

#[derive(serde::Deserialize, Component, Clone, Default)]
#[serde(try_from = "VertexRaw")]
pub struct Vertex {
    pub name: String,
    pub x: f64,
    pub y: f64,
}

impl TryFrom<VertexRaw> for Vertex {
    type Error = String;

    /// NOTE: This loads the vertex data "as is", in older maps, it will contain the raw
    /// "pixel coordinates" which needs to be converted to meters for the site map viewer
    /// to work correctly.
    fn try_from(raw: VertexRaw) -> Result<Vertex, Self::Error> {
        let x_raw = raw.data[0]
            .as_f64()
            .ok_or("expected first element to be a number")?;
        let y_raw = raw.data[1]
            .as_f64()
            .ok_or("expected second element to be a number")?;
        let name = if raw.data.len() > 3 {
            raw.data[3]
                .as_str()
                .ok_or("expected fourth element to be a string")?
                .to_string()
        } else {
            String::new()
        };
        Ok(Vertex {
            name,
            x: x_raw,
            y: y_raw,
        })
    }
}

impl Vertex {
    pub fn transform(&self) -> Transform {
        Transform {
            translation: Vec3::new(self.x as f32, self.y as f32, 0.),
            ..Default::default()
        }
    }
}

#[derive(serde::Deserialize)]
#[serde(transparent)]
struct VertexRaw {
    data: Vec<serde_yaml::Value>,
}

#[derive(serde::Deserialize)]
#[allow(dead_code)]
struct VertexProperties {
    is_charger: Option<(usize, bool)>,
    is_parking_spot: Option<(usize, bool)>,
    is_holding_point: Option<(usize, bool)>,
    spawn_robot_name: Option<(usize, String)>,
    spawn_robot_type: Option<(usize, String)>,
    dropoff_ingestor: Option<(usize, String)>,
    pickup_dispenser: Option<(usize, String)>,
}
