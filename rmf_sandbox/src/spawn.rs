// In order for saving the map to work correctly, the entities belonging to the site map
// must have ordered correctly in the hierarchy. Using the spawner ensures that the
// entities are spawned correctly.

use std::collections::HashMap;

use bevy::prelude::*;

use crate::{
    building_map::BuildingMap, lane::Lane, measurement::Measurement, model::Model, vertex::Vertex,
    wall::Wall,
};

pub struct SpawnInLevel<T> {
    level: String,
    obj: T,
}

#[derive(Component)]
struct LevelTag(String);

pub struct SiteMapRoot(Entity);

fn spawn_in_level<T: Component + Clone>(
    commands: &mut Commands,
    spawn: &mut EventReader<SpawnInLevel<T>>,
    levels: &HashMap<String, Entity>,
) {
    for data in spawn.iter() {
        if let Some(level_entity) = levels.get(&data.level) {
            commands.entity(*level_entity).with_children(|parent| {
                parent.spawn().insert(data.obj.clone());
            });
        }
    }
}

fn building_map_spawner(
    mut commands: Commands,
    map_root: Res<SiteMapRoot>,
    building_map: Option<Res<BuildingMap>>,
    mut vertices: EventWriter<SpawnInLevel<Vertex>>,
    mut lanes: EventWriter<SpawnInLevel<Lane>>,
    mut measurements: EventWriter<SpawnInLevel<Measurement>>,
    mut walls: EventWriter<SpawnInLevel<Wall>>,
    mut models: EventWriter<SpawnInLevel<Model>>,
) {
    let building_map = match building_map {
        Some(m) => {
            if !m.is_changed() && !m.is_added() {
                return;
            } else {
                m
            }
        }
        None => return,
    };

    commands.entity(map_root.0).despawn_descendants();
    for (name, level) in &building_map.levels {
        commands
            .spawn()
            .insert(LevelTag(name.clone()))
            .insert_bundle(TransformBundle::from_transform(Transform {
                translation: Vec3::new(0., 0., level.transform.translation[2] as f32),
                ..default()
            }))
            .insert(Parent(map_root.0));
        for vertex in &level.vertices {
            vertices.send(SpawnInLevel {
                level: name.clone(),
                obj: vertex.clone(),
            });
        }
        for lane in &level.lanes {
            lanes.send(SpawnInLevel {
                level: name.clone(),
                obj: lane.clone(),
            });
        }
        for measurement in &level.measurements {
            measurements.send(SpawnInLevel {
                level: name.clone(),
                obj: measurement.clone(),
            });
        }
        for wall in &level.walls {
            walls.send(SpawnInLevel {
                level: name.clone(),
                obj: wall.clone(),
            });
        }
        for model in &level.models {
            models.send(SpawnInLevel {
                level: name.clone(),
                obj: model.clone(),
            });
        }
    }
}

fn spawner(
    mut commands: Commands,
    levels: Query<(Entity, &LevelTag)>,
    mut vertices: EventReader<SpawnInLevel<Vertex>>,
    mut lanes: EventReader<SpawnInLevel<Lane>>,
    mut measurements: EventReader<SpawnInLevel<Measurement>>,
    mut walls: EventReader<SpawnInLevel<Wall>>,
    mut models: EventReader<SpawnInLevel<Model>>,
) {
    let levels: HashMap<String, Entity> = levels
        .iter()
        .map(|(entity, tag)| (tag.0.clone(), entity))
        .collect();

    spawn_in_level(&mut commands, &mut vertices, &levels);
    spawn_in_level(&mut commands, &mut lanes, &levels);
    spawn_in_level(&mut commands, &mut measurements, &levels);
    spawn_in_level(&mut commands, &mut walls, &levels);
    spawn_in_level(&mut commands, &mut models, &levels);
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
        app.add_event::<SpawnInLevel<Vertex>>()
            .add_event::<SpawnInLevel<Lane>>()
            .add_event::<SpawnInLevel<Measurement>>()
            .add_event::<SpawnInLevel<Wall>>()
            .add_event::<SpawnInLevel<Model>>()
            .add_startup_system(init_spawner)
            .add_system(building_map_spawner)
            .add_system(spawner);
    }
}
