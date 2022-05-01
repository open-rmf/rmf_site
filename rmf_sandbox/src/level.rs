use super::lane::Lane;
use super::site_map::Handles;
use super::vertex::Vertex;
use super::wall::Wall;
use bevy::prelude::*;
use serde_yaml;

#[derive(Component, Clone, Default)]
pub struct Level {
    pub vertices: Vec<Vertex>,
    pub lanes: Vec<Lane>,
    pub walls: Vec<Wall>,
    pub elevation: f32,
}

impl Level {
    pub fn spawn(
        &self,
        commands: &mut Commands,
        meshes: &mut ResMut<Assets<Mesh>>,
        handles: &Res<Handles>,
    ) {
        for v in &self.vertices {
            v.spawn(commands, handles, self.elevation);
        }

        for lane in &self.lanes {
            lane.spawn(&self.vertices, commands, meshes, handles, self.elevation);
        }

        for wall in &self.walls {
            wall.spawn(&self.vertices, commands, meshes, handles, self.elevation);
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

        level.elevation = match data["elevation"].as_f64() {
            Some(e) => e as f32,
            None => 0.,
        };

        // todo: calculate scale and inter-level alignment
        let mut ofs_x = 0.0;
        let mut ofs_y = 0.0;
        let scale = 1.0 / 100.0;
        let mut num_v = 0;
        for v in &level.vertices {
            ofs_x += v.x;
            ofs_y += v.y;
            num_v += 1;
        }
        ofs_x /= num_v as f64;
        ofs_y /= num_v as f64;
        for v in level.vertices.iter_mut() {
            v.x = (v.x - ofs_x) * scale;
            v.y = (v.y - ofs_y) * scale;
        }
        return level;
    }
}
