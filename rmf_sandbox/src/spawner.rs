// In order for saving the map to work correctly, the entities belonging to the site map
// must have ordered correctly in the hierarchy. Using the spawner ensures that the
// entities are spawned correctly.

use crate::{basic_components::Name, level::LevelDrawing};
use std::collections::HashMap;

use bevy::{
    ecs::system::{EntityCommands, SystemParam},
    prelude::*,
};

use crate::{
    building_map::BuildingMap, lane::Lane, measurement::Measurement, model::Model, vertex::Vertex,
    wall::Wall,
};

pub struct SiteMapRoot(pub Entity);

#[derive(Default)]
pub struct MapLevels(HashMap<String, Entity>);

#[derive(SystemParam)]
pub struct Spawner<'w, 's> {
    commands: Commands<'w, 's>,
    levels: ResMut<'w, MapLevels>,
    map_root: ResMut<'w, SiteMapRoot>,
}

#[derive(Component)]
pub struct LevelExtra {
    pub drawing: LevelDrawing,
    pub elevation: f64,
    pub flattened_x_offset: f64,
    pub flattened_y_offset: f64,
}

pub trait Spawnable: Component {}

impl Spawnable for Vertex {}
impl Spawnable for Lane {}
impl Spawnable for Measurement {}
impl Spawnable for Wall {}
impl Spawnable for Model {}

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

    /// Spawns a building map and all the spawnables inside it.
    pub fn spawn_map(&mut self, building_map: &BuildingMap) {
        self.commands
            .entity(self.map_root.0)
            .insert(Name(building_map.name.clone()))
            .insert(building_map.crowd_sim.clone())
            .despawn_descendants();
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
                .insert(Parent(self.map_root.0))
                .id();
            self.levels.0.insert(name.clone(), level_entity);

            for vertex in &level.vertices {
                self.spawn_in_level(name, vertex.clone());
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

fn init_spawner(mut commands: Commands) {
    let map_root = commands
        .spawn()
        .insert_bundle(TransformBundle::default())
        .id();
    commands.insert_resource(SiteMapRoot(map_root));
}

pub struct SpawnerPlugin;

impl Plugin for SpawnerPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MapLevels>()
            .add_startup_system(init_spawner);
    }
}
