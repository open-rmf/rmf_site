use super::lane::Lane;
use super::level_transform::LevelTransform;
use super::measurement::Measurement;
use super::site_map::Handles;
use super::vertex::Vertex;
use super::wall::Wall;
use bevy::prelude::*;
use serde_yaml;

#[derive(Component, Clone, Default)]
pub struct Level {
    pub vertices: Vec<Vertex>,
    pub lanes: Vec<Lane>,
    pub measurements: Vec<Measurement>,
    pub walls: Vec<Wall>,
    pub transform: LevelTransform,
}

impl Level {
    pub fn spawn(
        &self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        handles: &Res<Handles>,
    ) {
        for v in &self.vertices {
            v.spawn(commands, handles, &self.transform);
        }

        for lane in &self.lanes {
            lane.spawn(&self.vertices, commands, meshes, handles, &self.transform);
        }

        for measurement in &self.measurements {
            measurement.spawn(&self.vertices, commands, meshes, handles, &self.transform);
        }

        for wall in &self.walls {
            wall.spawn(&self.vertices, commands, meshes, handles, &self.transform);
        }

        // todo: use elevation
        commands.spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
            material: handles.default_floor_material.clone(),
            transform: Transform {
                rotation: Quat::from_rotation_x(1.57),
                ..Default::default()
            },
            ..Default::default()
        });
    }

    pub fn from_yaml(name: &str, data: &serde_yaml::Value) -> Level {
        println!("parsing level name: [{}]", name);
        let mut level = Level::default();
        for vertex_yaml in data["vertices"].as_sequence().unwrap() {
            level.vertices.push(Vertex::from_yaml(vertex_yaml));
        }
        for lane_yaml in data["lanes"].as_sequence().unwrap() {
            level.lanes.push(Lane::from_yaml(lane_yaml));
        }
        let walls_yaml = data["walls"].as_sequence();
        if walls_yaml.is_some() {
            for wall_yaml in walls_yaml.unwrap() {
                level.walls.push(Wall::from_yaml(wall_yaml));
            }
        }

        let meas_seq = data["measurements"].as_sequence();
        if meas_seq.is_some() {
            for meas in meas_seq.unwrap() {
                level.measurements.push(Measurement::from_yaml(meas));
            }
        }

        level.transform.translation[2] = match data["elevation"].as_f64() {
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
        return level;
    }
}
