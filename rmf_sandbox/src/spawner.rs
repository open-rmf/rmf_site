// In order for saving the map to work correctly, the entities belonging to the site map
// must have ordered correctly in the hierarchy. Using the spawner ensures that the
// entities are spawned correctly.

use crate::{
    basic_components::{Id, Name},
    crowd_sim::CrowdSim,
    despawn::PendingDespawn,
    door::Door,
    fiducial::Fiducial,
    floor::Floor,
    level::LevelDrawing,
    lift::Lift,
    camera::Camera,
};
use std::collections::HashMap;

use bevy::{
    ecs::system::{EntityCommands, SystemParam},
    prelude::*,
};

use crate::{
    building_map::BuildingMap, lane::Lane, light::Light, measurement::Measurement, model::Model,
    vertex::Vertex, wall::Wall,
};

#[derive(Component)]
pub struct SiteMapRoot;

#[derive(Default)]
pub struct MapLevels(HashMap<String, Entity>);

#[derive(Default)]
pub struct VerticesManagers(pub HashMap<String, LevelVerticesManager>);

#[derive(Clone, Default)]
pub struct LevelVerticesManager {
    id_to_entity: HashMap<usize, Entity>,
    entity_to_id: HashMap<Entity, usize>,
    next_id: usize,
}

impl LevelVerticesManager {
    pub fn add(&mut self, entity: Entity) -> usize {
        self.id_to_entity.insert(self.next_id, entity);
        self.entity_to_id.insert(entity, self.next_id);
        self.next_id += 1;
        self.next_id - 1
    }

    pub fn get(&self, id: usize) -> Option<Entity> {
        self.id_to_entity.get(&id).cloned()
    }

    pub fn get_entity(&self, entity: Entity) -> Option<usize> {
        self.entity_to_id.get(&entity).cloned()
    }

    pub fn remove(&mut self, id: usize) {
        match self.id_to_entity.get(&id) {
            Some(entity) => {
                // Delete
                self.entity_to_id.remove(entity);
                self.id_to_entity.remove(&id);
            }
            None => {}
        }
    }
}

#[derive(Component)]
pub struct LevelExtra {
    pub drawing: LevelDrawing,
    pub elevation: f64,
    pub flattened_x_offset: f64,
    pub flattened_y_offset: f64,
    pub fiducials: Vec<Fiducial>,
}

#[derive(Component)]
pub struct BuildingMapExtra {
    pub crowd_sim: CrowdSim,
}

pub trait Spawnable: Component {}

impl Spawnable for Floor {}
impl Spawnable for Lane {}
impl Spawnable for Light {}
impl Spawnable for Measurement {}
impl Spawnable for Wall {}
impl Spawnable for Model {}
impl Spawnable for Door {}
impl Spawnable for Lift {}
impl Spawnable for Camera {}

#[derive(SystemParam)]
pub struct Spawner<'w, 's> {
    commands: Commands<'w, 's>,
    levels: ResMut<'w, MapLevels>,
    map_root: Query<'w, 's, Entity, With<SiteMapRoot>>,
    vertex_mgrs: ResMut<'w, VerticesManagers>,
}

impl<'w, 's> Spawner<'w, 's> {
    pub fn spawn_in_level<T: Spawnable>(
        &mut self,
        level: &str,
        obj: T,
    ) -> Option<EntityCommands<'w, 's, '_>> {
        if let Some(level_entity) = self.levels.0.get(level) {
            let mut ec = self.commands.spawn();
            ec.insert(obj).insert(Parent(*level_entity));
            Some(ec)
        } else {
            println!("ERROR: Level {} not found", level);
            None
        }
    }

    pub fn spawn_vertex(
        &mut self,
        level: &str,
        vertex: Vertex,
    ) -> Option<EntityCommands<'w, 's, '_>> {
        if let Some(level_entity) = self.levels.0.get(level) {
            let mut ec = self.commands.spawn();
            let vertex_entity = ec.insert(vertex).insert(Parent(*level_entity)).id();
            let vm = self.vertex_mgrs.0.get_mut(level).unwrap();
            let id = vm.add(vertex_entity);
            ec.insert(Id(id));
            Some(ec)
        } else {
            println!("ERROR: Level {} not found", level);
            None
        }
    }

    /// Spawns a building map and all the spawnables inside it.
    pub fn spawn_map(&mut self, building_map: &BuildingMap) {
        for e in self.map_root.iter() {
            self.commands.entity(e).insert(PendingDespawn);
        }

        let map_root = self
            .commands
            .spawn()
            .insert(SiteMapRoot)
            .insert_bundle(TransformBundle::default())
            .insert(Name(building_map.name.clone()))
            .insert(BuildingMapExtra {
                crowd_sim: building_map.crowd_sim.clone(),
            })
            .id();

        self.vertex_mgrs.0.clear();
        self.levels.0.clear();
        for (name, level) in &building_map.levels {
            let level_entity = self
                .commands
                .spawn()
                .insert(Name(name.clone()))
                .insert_bundle(TransformBundle::from_transform(Transform {
                    translation: Vec3::new(0., 0., level.elevation as f32),
                    ..default()
                }))
                .insert(LevelExtra {
                    drawing: level.drawing.clone(),
                    elevation: level.elevation,
                    flattened_x_offset: level.flattened_x_offset,
                    flattened_y_offset: level.flattened_y_offset,
                    fiducials: level.fiducials.clone(),
                })
                .insert(Parent(map_root))
                .id();

            self.vertex_mgrs
                .0
                .insert(name.clone(), LevelVerticesManager::default());
            self.levels.0.insert(name.clone(), level_entity);

            for f in &level.floors {
                self.spawn_in_level(name, f.clone()).unwrap();
            }
            for vertex in &level.vertices {
                self.spawn_vertex(name, vertex.clone());
            }
            for lane in &level.lanes {
                self.spawn_in_level(name, lane.clone());
            }
            for measurement in &level.measurements {
                self.spawn_in_level(name, measurement.clone());
            }
            for wall in &level.walls {
                self.spawn_in_level(name, wall.clone());
            }
            for model in &level.models {
                self.spawn_in_level(name, model.clone());
            }
            for door in &level.doors {
                self.spawn_in_level(name, door.clone());
            }
            for camera in &level.cameras {
                self.spawn_in_level(name, camera.clone());
            }

            for (lift_name, lift) in &building_map.lifts {
                if lift.initial_floor_name == *name {
                    self.spawn_in_level(name, lift.clone())
                        .unwrap()
                        .insert(Name(lift_name.clone()));
                }
            }
        }
    }
}

pub struct SpawnerPlugin;

impl Plugin for SpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapLevels>()
            .init_resource::<VerticesManagers>();
    }
}
