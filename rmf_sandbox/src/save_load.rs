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
    door::Door,
    floor::Floor,
    lane::Lane,
    level::Level,
    lift::Lift,
    measurement::Measurement,
    model::Model,
    physical_camera::PhysicalCamera,
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
        Query<Entity, With<SiteMapRoot>>,
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
        Query<&Door>,
        Query<&PhysicalCamera>,
        Query<(&Name, &Lift)>,
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
        q_doors,
        q_physical_cameras,
        q_lifts,
    ) = state.get_mut(world);
    let root_entity = match root_entity.get_single() {
        Ok(root_entity) => root_entity,
        Err(err) => {
            println!("ERROR: Cannot save map ({})", err);
            return;
        }
    };

    let mut levels: BTreeMap<String, Level> = BTreeMap::new();

    let mut lifts: BTreeMap<String, Lift> = BTreeMap::new();
    for (lift_name, lift) in q_lifts.iter() {
        lifts.insert(lift_name.0.clone(), lift.clone());
    }

    for level in q_children.get(root_entity).unwrap().into_iter() {
        let mut floors: Vec<Floor> = Vec::new();
        let mut vertices: Vec<Vertex> = Vec::new();
        let mut lanes: Vec<Lane> = Vec::new();
        let mut measurements: Vec<Measurement> = Vec::new();
        let mut walls: Vec<Wall> = Vec::new();
        let mut models: Vec<Model> = Vec::new();
        let mut doors: Vec<Door> = Vec::new();
        let mut physical_cameras: Vec<PhysicalCamera> = Vec::new();
        let extra = q_level_extra.get(*level).unwrap();
        let name = q_name.get(*level).unwrap().0.clone();
        let vm = vms.0.get_mut(&name).unwrap();
        let mut new_vm = LevelVerticesManager::default();
        let mut vertex_file_id: HashMap<usize, usize> = HashMap::new();

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
                let new_id = new_vm.add(vm.id_to_entity(id).unwrap());
                vertex_file_id.insert(id, new_id);
                vertices.push(vertex.clone());
            }
        }
        *vm = new_vm.clone();

        for c in q_children.get(*level).unwrap().into_iter() {
            if let Ok(mut lane) = q_lanes.get_mut(*c) {
                lane.0 = vertex_file_id[&lane.0];
                lane.1 = vertex_file_id[&lane.1];
                lanes.push(lane.clone());
            }
            if let Ok(mut measurement) = q_measurements.get_mut(*c) {
                measurement.0 = vertex_file_id[&measurement.0];
                measurement.1 = vertex_file_id[&measurement.1];
                measurements.push(measurement.clone());
            }
            if let Ok(mut wall) = q_walls.get_mut(*c) {
                wall.0 = vertex_file_id[&wall.0];
                wall.1 = vertex_file_id[&wall.1];
                walls.push(wall.clone());
            }
            if let Ok(model) = q_models.get(*c) {
                models.push(model.clone());
            }
            if let Ok(door) = q_doors.get(*c) {
                doors.push(door.clone());
            }
            if let Ok(camera) = q_physical_cameras.get(*c) {
                physical_cameras.push(camera.clone());
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
                doors,
                physical_cameras,
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
        name: q_name.get(root_entity).unwrap().0.clone(),
        version: Some(2),
        lifts,
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

#[cfg(test)]
mod test {
    use super::*;
    use crate::spawner::*;

    #[test]
    fn test_save() -> Result<(), Box<dyn std::error::Error>> {
        let mut app = App::new();
        app.add_plugin(HierarchyPlugin)
            .add_plugin(SaveLoadPlugin)
            .add_plugin(SpawnerPlugin)
            .add_plugin(crate::despawn::DespawnPlugin);

        let buffer = std::fs::read("assets/demo_maps/office.building.yaml").unwrap();
        let map = BuildingMap::from_bytes(&buffer).unwrap();
        let cap_map = map.clone();
        app.add_system(move |mut spawner: Spawner, mut ran: Local<bool>| {
            if *ran {
                return;
            }
            spawner.spawn_map(&cap_map);
            *ran = true;
        });
        app.update();

        app.world
            .resource_mut::<Events<SaveMap>>()
            .send(SaveMap(PathBuf::from("test_output/save_map.yaml")));
        app.update();

        let buffer = std::fs::read("test_output/save_map.yaml").unwrap();
        let new_map = BuildingMap::from_bytes(&buffer).unwrap();

        assert!(new_map.levels.len() == map.levels.len());
        for (new_map, original_map) in new_map.levels.into_iter().zip(map.levels.into_iter()) {
            // good enough to check that nothing is missing, checking if all values are correct
            // would be quite complicated as units will be converted and items may be re-keyed.
            assert!(new_map.0 == original_map.0);
            assert!(new_map.1.vertices.len() == original_map.1.vertices.len());
            assert!(new_map.1.lanes.len() == original_map.1.lanes.len());
            assert!(new_map.1.measurements.len() == original_map.1.measurements.len());
            assert!(new_map.1.walls.len() == original_map.1.walls.len());
            assert!(new_map.1.models.len() == original_map.1.models.len());
        }

        Ok(())
    }
}
