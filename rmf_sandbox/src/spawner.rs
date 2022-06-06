// In order for saving the map to work correctly, the entities belonging to the site map
// must have ordered correctly in the hierarchy. Using the spawner ensures that the
// entities are spawned correctly.

use crate::{
    basic_components::{Id, Name},
    despawn::PendingDespawn,
    level::LevelDrawing,
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
    vertices: HashMap<usize, Entity>,
    next_id: usize,
}

impl LevelVerticesManager {
    pub fn add(&mut self, entity: Entity) -> usize {
        self.vertices.insert(self.next_id, entity);
        self.next_id += 1;
        self.next_id - 1
    }

    pub fn get(&self, id: usize) -> Option<Entity> {
        self.vertices.get(&id).cloned()
    }
}

#[derive(Component)]
pub struct LevelExtra {
    pub drawing: LevelDrawing,
    pub elevation: f64,
    pub flattened_x_offset: f64,
    pub flattened_y_offset: f64,
}

pub trait Spawnable: Component {}

impl Spawnable for Lane {}
impl Spawnable for Light {}
impl Spawnable for Measurement {}
impl Spawnable for Wall {}
impl Spawnable for Model {}

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
            .id();

        self.commands
            .entity(map_root)
            .insert(Name(building_map.name.clone()))
            .insert(building_map.crowd_sim.clone())
            .with_children(|_| {});
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
                })
                .insert(Parent(map_root))
                .id();
            self.vertex_mgrs
                .0
                .insert(name.clone(), LevelVerticesManager::default());
            self.levels.0.insert(name.clone(), level_entity);

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
