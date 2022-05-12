use crate::lane::Lane;
use crate::level::Level;
use crate::measurement::Measurement;
use crate::model::Model;
use crate::vertex::Vertex;
use crate::{building_map::BuildingMap, wall::Wall};
use bevy::{ecs::schedule::ShouldRun, prelude::*, transform::TransformBundle};

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
pub struct SiteMapLabel;

/// Event to spawn a vertex.
pub struct SpawnVertex(Vertex);
/// Event to spawn a lane.
pub struct SpawnLane(Lane);
/// Event to spawn a measurement.
pub struct SpawnMeasurement(Measurement);
/// Event to spawn a wall.
pub struct SpawnWall(Wall);
/// Event to spawn a model.
pub struct SpawnModel(Model);

#[derive(Default)]
struct Handles {
    pub default_floor_material: Handle<StandardMaterial>,
    pub lane_material: Handle<StandardMaterial>,
    pub measurement_material: Handle<StandardMaterial>,
    pub vertex_mesh: Handle<Mesh>,
    pub vertex_material: Handle<StandardMaterial>,
    pub wall_material: Handle<StandardMaterial>,
}

#[derive(Default, Component)]
pub struct SiteMap {
    pub site_name: String,
    pub levels: Vec<Level>,
}

impl SiteMap {
    pub fn from_building_map(building_map: BuildingMap) -> SiteMap {
        let sm = SiteMap {
            site_name: building_map.name,
            levels: building_map.levels.into_values().collect(),
            ..Default::default()
        };

        // todo: global alignment via fiducials

        return sm;
    }
}

/// Used to keep track of the entity that represents the current level being rendered by the plugin.
struct SiteMapLevel(Entity);

/// Used to keep track of entities created by the site map system.
#[derive(Component)]
struct SiteMapTag;

fn init_site_map(
    sm: Res<SiteMap>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut spawn_vertex: EventWriter<SpawnVertex>,
    mut spawn_lane: EventWriter<SpawnLane>,
    mut spawn_measurement: EventWriter<SpawnMeasurement>,
    mut spawn_wall: EventWriter<SpawnWall>,
    mut spawn_model: EventWriter<SpawnModel>,
) {
    println!("Loading assets");
    let mut handles = Handles::default();
    handles.vertex_mesh = meshes.add(Mesh::from(shape::Capsule {
        radius: 0.25,
        rings: 2,
        depth: 0.05,
        latitudes: 8,
        longitudes: 16,
        uv_profile: shape::CapsuleUvProfile::Fixed,
    }));
    handles.default_floor_material = materials.add(Color::rgb(0.3, 0.3, 0.3).into());
    handles.lane_material = materials.add(Color::rgb(1.0, 0.5, 0.3).into());
    handles.measurement_material = materials.add(Color::rgb(1.0, 0.5, 1.0).into());
    handles.vertex_material = materials.add(Color::rgb(0.4, 0.7, 0.6).into());
    handles.wall_material = materials.add(Color::rgb(0.5, 0.5, 1.0).into());

    println!("Initializing site map: {}", sm.site_name);
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.001,
    });

    commands
        .spawn_bundle(DirectionalLightBundle {
            directional_light: DirectionalLight {
                shadows_enabled: false,
                illuminance: 20000.,
                ..Default::default()
            },
            transform: Transform {
                translation: Vec3::new(0., 0., 50.),
                rotation: Quat::from_rotation_x(0.4),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(SiteMapTag);

    let level = &sm.levels[0];
    let vertices = &level.vertices;
    let entity = commands
        .spawn()
        .insert(SiteMapTag)
        .insert_bundle(TransformBundle::from_transform(Transform {
            translation: Vec3::new(0., 0., level.transform.translation[2] as f32),
            ..default()
        }))
        .id();
    commands.insert_resource(SiteMapLevel(entity));

    for v in vertices {
        spawn_vertex.send(SpawnVertex(v.clone()));
    }
    for lane in &level.lanes {
        spawn_lane.send(SpawnLane(lane.clone()));
    }
    for measurement in &level.measurements {
        spawn_measurement.send(SpawnMeasurement(measurement.clone()));
    }
    for wall in &level.walls {
        spawn_wall.send(SpawnWall(wall.clone()));
    }
    for model in &level.models {
        spawn_model.send(SpawnModel(model.clone()));
    }

    // spawn the floor plane
    // todo: use real floor polygons
    commands
        .spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
            material: handles.default_floor_material.clone(),
            transform: Transform {
                rotation: Quat::from_rotation_x(1.57),
                ..Default::default()
            },
            ..Default::default()
        })
        .insert(SiteMapTag);

    commands.insert_resource(vertices.clone());
    commands.insert_resource(handles);

    println!("Finished initializing site map");
}

fn despawn_site_map(mut commands: Commands, site_map_entities: Query<Entity, With<SiteMapTag>>) {
    println!("Unloading assets");
    // removing all the strong handles should automatically unload the assets.
    commands.remove_resource::<Handles>();
    // FIXME: removing this causes panick when unloading site map.
    // commands.remove_resource::<AmbientLight>();

    println!("Despawn all entites");
    for entity in site_map_entities.iter() {
        commands.entity(entity).despawn_recursive();
    }
    commands.remove_resource::<SiteMapLevel>();
}

fn update_vertices(
    mut commands: Commands,
    level_entity: Res<SiteMapLevel>,
    handles: Res<Handles>,
    mut vertices: ResMut<Vec<Vertex>>,
    mut spawn_vertex: EventReader<SpawnVertex>,
    mut changed_vertices: Query<(&Vertex, &mut Transform), Changed<Vertex>>,
) {
    // spawn new vertices
    for v in spawn_vertex.iter() {
        vertices.push(v.0.clone());
        commands.entity(level_entity.0).with_children(|cb| {
            cb.spawn_bundle(PbrBundle {
                mesh: handles.vertex_mesh.clone(),
                material: handles.vertex_material.clone(),
                transform: v.0.transform(),
                ..Default::default()
            })
            .insert(v.0.clone());
        });
    }
    // update changed vertices
    for (v, mut t) in changed_vertices.iter_mut() {
        *t = v.transform();
    }
}

fn update_lanes(
    mut commands: Commands,
    level_entity: Res<SiteMapLevel>,
    handles: Res<Handles>,
    mut meshes: ResMut<Assets<Mesh>>,
    vertices: Res<Vec<Vertex>>,
    mut spawn_lane: EventReader<SpawnLane>,
    mut changed_lanes: Query<(&Lane, &mut Transform), Changed<Lane>>,
) {
    // spawn new lanes
    for lane in spawn_lane.iter() {
        commands.entity(level_entity.0).with_children(|cb| {
            cb.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::from([1., 1.])))),
                material: handles.lane_material.clone(),
                transform: lane.0.transform(&vertices),
                ..Default::default()
            })
            .insert(lane.0.clone());
        });
    }
    // update changed lanes
    for (lane, mut t) in changed_lanes.iter_mut() {
        *t = lane.transform(&vertices);
    }
}

fn update_measurements(
    mut commands: Commands,
    level_entity: Res<SiteMapLevel>,
    handles: Res<Handles>,
    mut meshes: ResMut<Assets<Mesh>>,
    vertices: Res<Vec<Vertex>>,
    mut spawn_measurement: EventReader<SpawnMeasurement>,
    mut changed_measurements: Query<(&Measurement, &mut Transform), Changed<Measurement>>,
) {
    // spawn new measurements
    for measurement in spawn_measurement.iter() {
        commands.entity(level_entity.0).with_children(|cb| {
            cb.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::from([1., 1.])))),
                material: handles.measurement_material.clone(),
                transform: measurement.0.transform(&vertices),
                ..Default::default()
            })
            .insert(measurement.0.clone());
        });
    }
    // update changed measurements
    for (measurement, mut t) in changed_measurements.iter_mut() {
        *t = measurement.transform(&vertices);
    }
}

fn update_walls(
    mut commands: Commands,
    level_entity: Res<SiteMapLevel>,
    handles: Res<Handles>,
    mut meshes: ResMut<Assets<Mesh>>,
    vertices: Res<Vec<Vertex>>,
    mut spawn_wall: EventReader<SpawnWall>,
    mut changed_walls: Query<(&Wall, &mut Transform), Changed<Wall>>,
) {
    // spawn new walls
    for wall in spawn_wall.iter() {
        commands.entity(level_entity.0).with_children(|cb| {
            cb.spawn_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Box::new(1., 1., 1.))),
                material: handles.wall_material.clone(),
                transform: wall.0.transform(&vertices),
                ..Default::default()
            })
            .insert(wall.0.clone());
        });
    }
    // update changed walls
    for (wall, mut t) in changed_walls.iter_mut() {
        *t = wall.transform(&vertices);
    }
}

fn update_models(
    mut commands: Commands,
    level_entity: Res<SiteMapLevel>,
    mut spawn_model: EventReader<SpawnModel>,
    mut changed_models: Query<(&Model, &mut Transform), (Changed<Model>, With<Model>)>,
    asset_server: Res<AssetServer>,
) {
    // spawn new models
    #[cfg(not(target_arch = "wasm32"))]
    for model in spawn_model.iter() {
        let bundle_path = String::from("http://models.sandbox.open-rmf.org/models/")
            + &model.0.model_name
            + &String::from(".glb#Scene0");
        println!(
            "spawning {} at {}, {}",
            &bundle_path, model.0.x_meters, model.0.y_meters
        );
        let glb = asset_server.load(&bundle_path);
        commands.entity(level_entity.0).with_children(|cb| {
            cb.spawn_bundle((model.0.transform(), GlobalTransform::identity()))
                .with_children(|parent| {
                    parent.spawn_scene(glb);
                })
                .insert(model.0.clone());
        });
    }
    // update changed models
    for (model, mut t) in changed_models.iter_mut() {
        *t = model.transform();
    }
}

fn has_site_map(level_entity: Option<Res<SiteMapLevel>>) -> ShouldRun {
    if level_entity.is_some() {
        return ShouldRun::Yes;
    }
    return ShouldRun::No;
}

fn should_init_site_map(sm: Option<Res<SiteMap>>) -> ShouldRun {
    if let Some(sm) = sm {
        if sm.is_added() {
            return ShouldRun::Yes;
        }
    }
    ShouldRun::No
}

fn should_despawn_site_map(sm: Option<Res<SiteMap>>, mut sm_existed: Local<bool>) -> ShouldRun {
    if sm.is_none() && *sm_existed {
        *sm_existed = false;
        return ShouldRun::Yes;
    }
    *sm_existed = sm.is_some();
    return ShouldRun::No;
}

#[derive(Default)]
pub struct SiteMapPlugin;

impl Plugin for SiteMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Vec<Vertex>>()
            .init_resource::<Handles>()
            .add_event::<SpawnVertex>()
            .add_event::<SpawnLane>()
            .add_event::<SpawnMeasurement>()
            .add_event::<SpawnWall>()
            .add_event::<SpawnModel>()
            .add_system_set(
                SystemSet::new()
                    .label(SiteMapLabel)
                    .with_system(init_site_map.with_run_criteria(should_init_site_map))
                    .with_system(despawn_site_map.with_run_criteria(should_despawn_site_map)),
            )
            .add_system_set(
                SystemSet::new()
                    .label(SiteMapLabel)
                    .with_run_criteria(has_site_map)
                    .with_system(update_vertices.after(init_site_map))
                    .with_system(update_lanes.after(update_vertices))
                    .with_system(update_walls.after(update_vertices))
                    .with_system(update_measurements.after(update_vertices))
                    .with_system(update_models.after(init_site_map)),
            );
    }
}
