use std::{collections::BTreeMap, path::PathBuf};

use bevy::{ecs::event::Events, prelude::*};

use crate::{
    basic_components::Name,
    building_map::BuildingMap,
    crowd_sim::CrowdSim,
    lane::Lane,
    level::Level,
    measurement::Measurement,
    model::Model,
    spawner::{LevelExtra, SiteMapRoot},
    vertex::Vertex,
    wall::Wall,
};

pub struct SaveLoadPlugin;

pub struct SaveMap(pub PathBuf);

/// The building map must be spawned through `SpawnerPlugin` for the data to be saved correctly.
fn save(world: &mut World) {
    let mut save_events = world.resource_mut::<Events<SaveMap>>();
    // if there are multiple save events for whatever reason, just process the last event.
    let path = match save_events.drain().last() {
        Some(SaveMap(path)) => path,
        None => return,
    };

    println!("Saving to {}", path.to_str().unwrap());

    let mut q_vertices = world.query::<&Vertex>();
    let mut q_lanes = world.query::<&Lane>();
    let mut q_measurements = world.query::<&Measurement>();
    let mut q_walls = world.query::<&Wall>();
    let mut q_models = world.query::<&Model>();

    let root_entity = world.entity(world.resource::<SiteMapRoot>().0);
    let crowd_sim = root_entity.get::<CrowdSim>().unwrap();
    let mut levels: BTreeMap<String, Level> = BTreeMap::new();

    for level in root_entity.get::<Children>().unwrap().iter() {
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut lanes: Vec<Lane> = Vec::new();
        let mut measurements: Vec<Measurement> = Vec::new();
        let mut walls: Vec<Wall> = Vec::new();
        let mut models: Vec<Model> = Vec::new();
        let extra = world.entity(*level).get::<LevelExtra>().unwrap();
        let name = world.get::<Name>(*level).unwrap().0.clone();
        for c in world.entity(*level).get::<Children>().unwrap().into_iter() {
            if let Ok(vertex) = q_vertices.get(world, *c) {
                vertices.push(vertex.clone());
            }
            if let Ok(lane) = q_lanes.get(world, *c) {
                lanes.push(lane.clone());
            }
            if let Ok(measurement) = q_measurements.get(world, *c) {
                measurements.push(measurement.clone());
            }
            if let Ok(wall) = q_walls.get(world, *c) {
                walls.push(wall.clone());
            }
            if let Ok(model) = q_models.get(world, *c) {
                models.push(model.clone());
            }
        }
        levels.insert(
            name,
            Level {
                vertices,
                lanes,
                measurements,
                walls,
                models,
                drawing: extra.drawing.clone(),
                elevation: extra.elevation,
                flattened_x_offset: extra.flattened_x_offset,
                flattened_y_offset: extra.flattened_y_offset,
            },
        );
    }

    let map = BuildingMap {
        name: root_entity.get::<Name>().unwrap().0.clone(),
        crowd_sim: crowd_sim.clone(),
        levels,
        ..default()
    };
    let f = std::fs::File::create(path).unwrap();
    serde_yaml::to_writer(f, &map).unwrap();
}

impl Plugin for SaveLoadPlugin {
    fn build(&self, app: &mut App) {
        app.add_event::<SaveMap>()
            .add_system(save.exclusive_system());
    }
}
