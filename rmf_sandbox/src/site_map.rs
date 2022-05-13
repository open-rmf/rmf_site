use std::collections::HashSet;

use crate::lane::Lane;
use crate::level::Level;
use crate::measurement::Measurement;
use crate::model::Model;
use crate::vertex::Vertex;
use crate::{building_map::BuildingMap, wall::Wall};
use bevy::ecs::system::SystemParam;
use bevy::{ecs::schedule::ShouldRun, prelude::*, transform::TransformBundle};

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
pub struct SiteMapLabel;

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

/// Keeps track of the entities of vertices.
#[derive(SystemParam)]
struct VerticesManager<'w, 's> {
    data: ResMut<'w, VerticesManagerData>,
    query: Query<'w, 's, (&'static Vertex, ChangeTrackers<Vertex>)>,
}

#[derive(Default)]
struct VerticesManagerData {
    entities: Vec<Entity>,
    used_by_entites: Vec<HashSet<Entity>>,
}

impl<'w, 's> VerticesManager<'w, 's> {
    fn push_vertex(&mut self, e: Entity, used_by: &[Entity]) {
        self.data.entities.push(e);
        self.data
            .used_by_entites
            .push(HashSet::from_iter(used_by.into_iter().cloned()));
    }

    fn insert_used_by(&mut self, vertex_id: usize, used_entity: Entity) {
        self.data.used_by_entites[vertex_id].insert(used_entity);
    }

    fn get_vertex(&self, vertex_id: usize) -> (&Vertex, ChangeTrackers<Vertex>) {
        self.query.get(self.data.entities[vertex_id]).unwrap()
    }
}

#[derive(Component, Default)]
struct VertexUsedBy(Vec<Entity>);

#[derive(Component, Default)]
struct VertexChanged(usize);

fn init_site_map(
    sm: Res<SiteMap>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
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

    commands.init_resource::<VerticesManagerData>();

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
        .with_children(|cb| {
            for v in vertices {
                cb.spawn().insert(v.clone());
            }
            for lane in &level.lanes {
                cb.spawn().insert(lane.clone());
            }
            for measurement in &level.measurements {
                cb.spawn().insert(measurement.clone());
            }
            for wall in &level.walls {
                cb.spawn().insert(wall.clone());
            }
            for model in &level.models {
                cb.spawn().insert(model.clone());
            }
        })
        .id();
    commands.insert_resource(SiteMapLevel(entity));

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
    // removing this causes bevy to panic, instead just replace it with the default.
    commands.init_resource::<AmbientLight>();

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
    mut vertices_mgr: VerticesManager,
    added_vertices: Query<(Entity, &Vertex), Added<Vertex>>,
    mut changed_vertices: Query<(&Vertex, &mut Transform), Changed<Vertex>>,
) {
    // spawn new vertices
    for (e, v) in added_vertices.iter() {
        commands
            .entity(e)
            .insert_bundle(PbrBundle {
                mesh: handles.vertex_mesh.clone(),
                material: handles.vertex_material.clone(),
                transform: v.transform(),
                ..Default::default()
            })
            .insert(Parent(level_entity.0));
        vertices_mgr.push_vertex(e, &[]);
    }
    // update changed vertices
    for (v, mut t) in changed_vertices.iter_mut() {
        *t = v.transform();
    }
}

fn update_lanes(
    mut commands: Commands,
    level_entity: Res<SiteMapLevel>,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<Handles>,
    vertices_mgr: VerticesManager,
    mut lanes: Query<(Entity, &Lane, ChangeTrackers<Lane>, Option<&mut Transform>)>,
) {
    // spawn new lanes
    for (e, lane, change, t) in lanes.iter_mut() {
        let (v1, v1_change) = vertices_mgr.get_vertex(lane.start);
        let (v2, v2_change) = vertices_mgr.get_vertex(lane.end);

        if change.is_added() {
            commands
                .entity(e)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::from([1., 1.])))),
                    material: handles.lane_material.clone(),
                    transform: lane.transform(v1, v2),
                    ..Default::default()
                })
                .insert(lane.clone())
                .insert(Parent(level_entity.0));
        } else if change.is_changed() || v1_change.is_changed() || v2_change.is_changed() {
            *t.unwrap() = lane.transform(v1, v2);
        }
    }
}

fn update_measurements(
    mut commands: Commands,
    level_entity: Res<SiteMapLevel>,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<Handles>,
    vertices_mgr: VerticesManager,
    mut measurements: Query<
        (
            Entity,
            &Measurement,
            ChangeTrackers<Measurement>,
            Option<&mut Transform>,
        ),
        Changed<Measurement>,
    >,
) {
    // spawn new measurements
    for (e, measurement, change, t) in measurements.iter_mut() {
        let (v1, v1_change) = vertices_mgr.get_vertex(measurement.start);
        let (v2, v2_change) = vertices_mgr.get_vertex(measurement.end);

        if change.is_added() {
            commands
                .entity(e)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::from([1., 1.])))),
                    material: handles.measurement_material.clone(),
                    transform: measurement.transform(v1, v2),
                    ..Default::default()
                })
                .insert(measurement.clone())
                .insert(Parent(level_entity.0));
        } else if change.is_changed() || v1_change.is_changed() || v2_change.is_changed() {
            *t.unwrap() = measurement.transform(v1, v2);
        }
    }
}

fn update_walls(
    mut commands: Commands,
    level_entity: Res<SiteMapLevel>,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<Handles>,
    mut vertices_mgr: VerticesManager,
    mut walls: Query<(Entity, &Wall, ChangeTrackers<Wall>, Option<&mut Transform>)>,
) {
    // spawn new walls
    for (e, wall, change, t) in walls.iter_mut() {
        let (v1, v1_change) = vertices_mgr.get_vertex(wall.start);
        let (v2, v2_change) = vertices_mgr.get_vertex(wall.end);

        if change.is_added() {
            commands
                .entity(e)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Box::new(1., 1., 1.))),
                    material: handles.wall_material.clone(),
                    transform: wall.transform(v1, v2),
                    ..Default::default()
                })
                .insert(wall.clone())
                .insert(Parent(level_entity.0));
            vertices_mgr.insert_used_by(wall.start, e);
            vertices_mgr.insert_used_by(wall.end, e);
        } else if change.is_changed() || v1_change.is_changed() || v2_change.is_changed() {
            *t.unwrap() = wall.transform(v1, v2);
        }
    }
}

fn update_models(
    mut commands: Commands,
    level_entity: Res<SiteMapLevel>,
    added_models: Query<(Entity, &Model), Added<Model>>,
    mut changed_models: Query<(&Model, &mut Transform), (Changed<Model>, With<Model>)>,
    asset_server: Res<AssetServer>,
) {
    // spawn new models
    #[cfg(not(target_arch = "wasm32"))]
    {
        for (e, model) in added_models.iter() {
            let bundle_path = String::from("http://models.sandbox.open-rmf.org/models/")
                + &model.model_name
                + &String::from(".glb#Scene0");
            println!(
                "spawning {} at {}, {}",
                &bundle_path, model.x_meters, model.y_meters
            );
            let glb = asset_server.load(&bundle_path);
            commands
                .entity(e)
                .insert_bundle((model.transform(), GlobalTransform::identity()))
                .with_children(|parent| {
                    parent.spawn_scene(glb);
                })
                .insert(model.clone())
                .insert(Parent(level_entity.0));
        }
    }
    // update changed models
    for (model, mut t) in changed_models.iter_mut() {
        *t = model.transform();
    }
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

fn has_site_map(level_entity: Option<Res<SiteMapLevel>>) -> ShouldRun {
    if level_entity.is_some() {
        return ShouldRun::Yes;
    }
    ShouldRun::No
}

#[derive(Default)]
pub struct SiteMapPlugin;

impl Plugin for SiteMapPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<Vec<Vertex>>()
            .init_resource::<Handles>()
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
