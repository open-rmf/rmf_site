use bevy::prelude::*;

use std::{
    env,
    fs::{File, metadata},
};

use serde_yaml;

// todo: use asset-server or something more sophisticated eventually.
// for now, just hack it up and toss the office-demo YAML into a big string
use super::demo_world::demo_office;

////////////////////////////////////////////////////////
// A few helper structs to use when parsing YAML files
////////////////////////////////////////////////////////

struct Vertex {
    x: f64,
    y: f64,
    _name: String,
}

struct Lane {
    start: usize,
    end: usize,
}

struct Wall {
    start: usize,
    end: usize,
}

struct SiteMap {
    site_name: String,
    vertices: Vec<Vertex>,
    lanes: Vec<Lane>,
    walls: Vec<Wall>,
}

impl Default for SiteMap {
    fn default() -> Self {
        SiteMap {
            site_name: String::new(),
            vertices: Vec::new(),
            lanes: Vec::new(),
            walls: Vec::new(),
        }
    }
}

////////////////////////////////////////////////////////
// A few events to use when requesting to spawn a map
////////////////////////////////////////////////////////

pub struct SpawnSiteMapFilename {
    pub filename: String,
}

pub struct SpawnSiteMapYaml {
    pub yaml_doc: serde_yaml::Value,
}

pub fn spawn_site_map_filename(
    mut ev_filename: EventReader<SpawnSiteMapFilename>,
    mut ev_yaml: EventWriter<SpawnSiteMapYaml>
) {
    for ev in ev_filename.iter() {
        let filename = &ev.filename;
        println!("spawn_site_map_filename: : [{}]", filename);
        if !metadata(&filename).is_ok() {
            println!("could not open [{}]", &filename);
            return;
        }
        let file = File::open(&filename).expect("Could not open file");
        let doc: serde_yaml::Value = serde_yaml::from_reader(file).ok().unwrap();
        ev_yaml.send(SpawnSiteMapYaml { yaml_doc: doc });
    }
}

pub fn spawn_site_map_yaml(
    mut ev_spawn: EventReader<SpawnSiteMapYaml>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    _asset_server: Res<AssetServer>,
    mesh_query: Query<(Entity, &Handle<Mesh>)>,
) {
    for ev in ev_spawn.iter() {
        let doc = &ev.yaml_doc;

        // first, despawn all existing mesh entities
        println!("despawing all meshes...");
        for entity_mesh in mesh_query.iter() {
            let (entity, _mesh) = entity_mesh;
            commands.entity(entity).despawn_recursive();
        }

        // parse the file into this object
        let mut sm = SiteMap {
            ..Default::default()
        };

        sm.site_name = doc["name"].as_str().unwrap().to_string();
        for (k, level_yaml) in doc["levels"].as_mapping().unwrap().iter() { //.iter() {
            println!("level name: [{}]", k.as_str().unwrap());
            for vertex_yaml in level_yaml["vertices"].as_sequence().unwrap() {
                let data = vertex_yaml.as_sequence().unwrap();
                let x = data[0].as_f64().unwrap();
                let y = data[1].as_f64().unwrap();
                let name = if data.len() > 3 { data[3].as_str().unwrap().to_string() } else { String::new() };
                let v = Vertex {
                    x: x,
                    y: -y,
                    _name: name
                };
                sm.vertices.push(v);
            }
            for lane_yaml in level_yaml["lanes"].as_sequence().unwrap() {
                let data = lane_yaml.as_sequence().unwrap();
                let start = data[0].as_u64().unwrap();
                let end = data[1].as_u64().unwrap();
                let lane = Lane {
                    start: start as usize,
                    end: end as usize
                };
                sm.lanes.push(lane);
            }
            let walls_yaml = level_yaml["walls"].as_sequence();
            if walls_yaml.is_some() {
                for wall_yaml in walls_yaml.unwrap() {
                    let data = wall_yaml.as_sequence().unwrap();
                    let start = data[0].as_u64().unwrap();
                    let end = data[1].as_u64().unwrap();
                    let wall = Wall {
                        start: start as usize,
                        end: end as usize
                    };
                    sm.walls.push(wall);
                }
            }
        }

        // todo: calculate scale and inter-level alignment
        let mut ofs_x = 0.0;
        let mut ofs_y = 0.0;
        let scale = 1.0 / 100.0;
        let mut num_v = 0;
        for v in &sm.vertices {
            ofs_x += v.x;
            ofs_y += v.y;
            num_v += 1;
        }
        ofs_x /= num_v as f64;
        ofs_y /= num_v as f64;

        // now spawn the file into the scene
        let vertex_handle = meshes.add(
            Mesh::from(
                shape::Capsule {
                    radius: 0.25,
                    rings: 2,
                    depth: 0.05,
                    latitudes: 8,
                    longitudes: 16,
                    uv_profile: shape::CapsuleUvProfile::Fixed,
                }
            )
        );

        let vertex_material_handle = materials.add(Color::rgb(0.4, 0.7, 0.6).into());

        for v in &sm.vertices {
            commands.spawn_bundle(PbrBundle {
                mesh: vertex_handle.clone(),
                material: vertex_material_handle.clone(),
                transform: Transform {
                    translation: Vec3::new(
                        ((v.x - ofs_x) * scale) as f32,
                        ((v.y - ofs_y) * scale) as f32,
                        0.0,
                    ),
                    rotation: Quat::from_rotation_x(1.57),
                    ..Default::default()
                },
                ..Default::default()
            });
        }

        let lane_material_handle = materials.add(Color::rgba(1.0, 0.5, 0.3, 0.5).into());

        let mut z_ofs = 0.01;
        for lane in &sm.lanes {
            let v1 = &sm.vertices[lane.start];
            let v2 = &sm.vertices[lane.end];
            let v1x = ((v1.x - ofs_x) * scale) as f32;
            let v1y = ((v1.y - ofs_y) * scale) as f32;
            let v2x = ((v2.x - ofs_x) * scale) as f32;
            let v2y = ((v2.y - ofs_y) * scale) as f32;

            let dx = v2x - v1x;
            let dy = v2y - v1y;
            let length = Vec2::from([dx, dy]).length();
            let width = 0.5 as f32;
            let yaw = dy.atan2(dx);
            let cx = (v1x + v2x) / 2.;
            let cy = (v1y + v2y) / 2.;

            commands.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::from([length, width])))),
                material: lane_material_handle.clone(),
                transform: Transform {
                    translation: Vec3::new(cx, cy, z_ofs),
                    rotation: Quat::from_rotation_z(yaw),
                    ..Default::default()
                },
                ..Default::default()
            });
            z_ofs += 0.001;  // avoid flicker
        }

        let wall_material_handle = materials.add(Color::rgb(0.5, 0.5, 1.0).into());

        for wall in &sm.walls {
            let v1 = &sm.vertices[wall.start];
            let v2 = &sm.vertices[wall.end];
            let v1x = ((v1.x - ofs_x) * scale) as f32;
            let v1y = ((v1.y - ofs_y) * scale) as f32;
            let v2x = ((v2.x - ofs_x) * scale) as f32;
            let v2y = ((v2.y - ofs_y) * scale) as f32;

            let dx = v2x - v1x;
            let dy = v2y - v1y;
            let length = Vec2::from([dx, dy]).length();
            let width = 0.1 as f32;
            let height = 1.0 as f32;
            let yaw = dy.atan2(dx);
            let cx = (v1x + v2x) / 2.;
            let cy = (v1y + v2y) / 2.;

            commands.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box::new(length, width, height))),
                material: wall_material_handle.clone(),
                transform: Transform {
                    translation: Vec3::new(cx, cy, height / 2.),
                    rotation: Quat::from_rotation_z(yaw),
                    ..Default::default()
                },
                ..Default::default()
            });
        }

    }
}

////////////////////////////////////////////////////////
// When starting up, either load the requested filename,
// or load a built-in demo map (the OSRC-SG office).
////////////////////////////////////////////////////////

pub fn initialize_site_map(
    mut spawn_yaml_writer: EventWriter<SpawnSiteMapYaml>,
    mut spawn_filename_writer: EventWriter<SpawnSiteMapFilename>,
) {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        spawn_filename_writer.send(SpawnSiteMapFilename { filename: args[1].clone() });
    } else {
        // load the office demo that is hard-coded in demo_world.rs
        let result: serde_yaml::Result<serde_yaml::Value> = serde_yaml::from_str(&demo_office());
        if result.is_err() {
            println!("serde threw an error: {:?}", result.err());
        }
        else {
            let doc: serde_yaml::Value = serde_yaml::from_str(&demo_office()).ok().unwrap();
            spawn_yaml_writer.send(SpawnSiteMapYaml { yaml_doc: doc });
        }
    }
}

#[derive(Default)]
pub struct SiteMapPlugin;

impl Plugin for SiteMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SpawnSiteMapYaml>()
           .add_event::<SpawnSiteMapFilename>()
           .add_startup_system(initialize_site_map)
           .add_system(spawn_site_map_yaml)
           .add_system(spawn_site_map_filename);
    }
}
