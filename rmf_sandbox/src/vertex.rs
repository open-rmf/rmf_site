use bevy::prelude::*;

#[derive(serde::Deserialize, Component, Clone, Default)]
#[serde(try_from = "VertexRaw")]
pub struct Vertex {
    pub name: String,
    pub x: f64,
    pub y: f64,
    pub z: f64,
    pub is_charger: bool,
    pub is_holding_point: bool,
    pub is_parking_spot: bool,
    pub spawn_robot_name: String,
    pub spawn_robot_type: String,
    pub dropoff_ingestor: String,
    pub pickup_dispenser: String,
}

impl TryFrom<VertexRaw> for Vertex {
    type Error = String;

    /// NOTE: This loads the vertex data "as is", in older maps, it will contain the raw
    /// "pixel coordinates" which needs to be converted to meters for the site map viewer
    /// to work correctly.
    fn try_from(raw: VertexRaw) -> Result<Vertex, Self::Error> {
        Ok(Vertex {
            x: raw.0,
            y: raw.1,
            z: raw.2,
            name: raw.3,
            is_charger: raw.4.is_charger.map_or(false, |x| x.1),
            is_holding_point: raw.4.is_holding_point.map_or(false, |x| x.1),
            is_parking_spot: raw.4.is_parking_spot.map_or(false, |x| x.1),
            spawn_robot_name: raw.4.spawn_robot_name.map_or("".to_string(), |x| x.1),
            spawn_robot_type: raw.4.spawn_robot_type.map_or("".to_string(), |x| x.1),
            dropoff_ingestor: raw.4.dropoff_ingestor.map_or("".to_string(), |x| x.1),
            pickup_dispenser: raw.4.pickup_dispenser.map_or("".to_string(), |x| x.1),
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

#[derive(serde::Deserialize, Default)]
struct VertexProperties {
    is_charger: Option<(usize, bool)>,
    is_parking_spot: Option<(usize, bool)>,
    is_holding_point: Option<(usize, bool)>,
    spawn_robot_name: Option<(usize, String)>,
    spawn_robot_type: Option<(usize, String)>,
    dropoff_ingestor: Option<(usize, String)>,
    pickup_dispenser: Option<(usize, String)>,
}

#[derive(serde::Deserialize)]
struct VertexRaw(
    f64,
    f64,
    f64,
    String,
    #[serde(default)] VertexProperties,
);
