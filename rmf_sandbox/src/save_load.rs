use std::{
    collections::{BTreeMap, HashMap},
    path::PathBuf,
};

use bevy::{
    ecs::{event::Events, system::SystemState},
    prelude::*,
};

use crate::{
    basic_components::{Id, Name},
    building_map::BuildingMap,
    floor::Floor,
    lane::Lane,
    level::Level,
    measurement::Measurement,
    model::Model,
    spawner::{BuildingMapExtra, LevelExtra, LevelVerticesManager, SiteMapRoot, VerticesManagers},
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

    let mut state: SystemState<(
        Res<SiteMapRoot>,
        Query<&Children>,
        Query<&BuildingMapExtra>,
        Query<&LevelExtra>,
        Query<&Name>,
        Query<&Id>,
        Query<&Floor>,
        ResMut<VerticesManagers>,
        Query<&Vertex>,
        Query<&mut Lane>,
        Query<&mut Measurement>,
        Query<&mut Wall>,
        Query<&Model>,
    )> = SystemState::new(world);
    let (
        root_entity,
        q_children,
        q_building_map_extra,
        q_level_extra,
        q_name,
        q_id,
        q_floors,
        mut vms,
        q_vertices,
        mut q_lanes,
        mut q_measurements,
        mut q_walls,
        q_models,
    ) = state.get_mut(world);

    let mut levels: BTreeMap<String, Level> = BTreeMap::new();

    for level in q_children.get(root_entity.0).unwrap().into_iter() {
        let mut floors: Vec<Floor> = Vec::new();
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut lanes: Vec<Lane> = Vec::new();
        let mut measurements: Vec<Measurement> = Vec::new();
        let mut walls: Vec<Wall> = Vec::new();
        let mut models: Vec<Model> = Vec::new();
        let extra = q_level_extra.get(*level).unwrap();
        let name = q_name.get(*level).unwrap().0.clone();
        let vm = vms.0.get_mut(&name).unwrap();
        let mut new_vm = LevelVerticesManager::default();
        let mut vertices_reid: HashMap<usize, usize> = HashMap::new();

        for c in q_children.get(*level).unwrap().into_iter() {
            if let Ok(floor) = q_floors.get(*c) {
                floors.push(floor.clone());
            }

            // Because the building format stores vertices as an array, with the id as the index,
            // all ids must have sequential numbers. During the cause of traffic editing, it is
            // possible for ids to be skipped if there are deletion operations so we need to
            // re-key all vertices when saving.
            if let Ok(vertex) = q_vertices.get(*c) {
                let id = q_id.get(*c).unwrap().0;
                let new_id = new_vm.add(vm.get(id).unwrap());
                vertices_reid.insert(id, new_id);
                vertices.push(vertex.clone());
            }
        }
        *vm = new_vm.clone();

        for c in q_children.get(*level).unwrap().into_iter() {
            if let Ok(mut lane) = q_lanes.get_mut(*c) {
                lane.0 = vertices_reid[&lane.0];
                lane.1 = vertices_reid[&lane.1];
                lanes.push(lane.clone());
            }
            if let Ok(mut measurement) = q_measurements.get_mut(*c) {
                measurement.0 = vertices_reid[&measurement.0];
                measurement.1 = vertices_reid[&measurement.1];
                measurements.push(measurement.clone());
            }
            if let Ok(mut wall) = q_walls.get_mut(*c) {
                wall.0 = vertices_reid[&wall.0];
                wall.1 = vertices_reid[&wall.1];
                walls.push(wall.clone());
            }
            if let Ok(model) = q_models.get(*c) {
                models.push(model.clone());
            }
        }
        levels.insert(
            name,
            Level {
                floors,
                vertices,
                lanes,
                measurements,
                walls,
                models,
                doors: extra.doors.clone(),
                drawing: extra.drawing.clone(),
                elevation: extra.elevation,
                flattened_x_offset: extra.flattened_x_offset,
                flattened_y_offset: extra.flattened_y_offset,
                fiducials: extra.fiducials.clone(),
            },
        );
    }

    let building_map_extra = q_building_map_extra.single();
    let map = BuildingMap {
        name: q_name.get(root_entity.0).unwrap().0.clone(),
        version: Some(2),
        lifts: building_map_extra.lifts.clone(),
        crowd_sim: building_map_extra.crowd_sim.clone(),
        levels,
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
