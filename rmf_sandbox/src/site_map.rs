use std::collections::{HashMap, HashSet};

use crate::despawn::DespawnBlocker;
use crate::lane::Lane;
use crate::level::Level;
use crate::measurement::Measurement;
use crate::model::Model;
use crate::vertex::Vertex;
use crate::{building_map::BuildingMap, wall::Wall};
use bevy::asset::LoadState;
use bevy::ecs::system::SystemParam;
use bevy::{ecs::schedule::ShouldRun, prelude::*, transform::TransformBundle};

#[derive(Clone, Hash, Debug, PartialEq, Eq, SystemLabel)]
pub struct SiteMapLabel;

#[derive(Default)]
pub struct MaterialMap {
    pub materials: HashMap<String, Handle<StandardMaterial>>,
}

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
    asset_server: Res<AssetServer>,
) {
    println!("Loading assets");
    let mut handles = Handles::default();
    handles.vertex_mesh = meshes.add(Mesh::from(shape::Capsule {
        radius: 0.15,
        rings: 2,
        depth: 0.05,
        latitudes: 8,
        longitudes: 16,
        uv_profile: shape::CapsuleUvProfile::Fixed,
    }));
    //handles.default_floor_material = materials.add(Color::rgb(0.3, 0.3, 0.3).into());
    handles.default_floor_material = materials.add(StandardMaterial {
        base_color: Color::rgb(0.3, 0.3, 0.3).into(),
        perceptual_roughness: 0.5,
        ..default()
    });
    handles.lane_material = materials.add(Color::rgb(1.0, 0.5, 0.3).into());
    handles.measurement_material = materials.add(Color::rgb(1.0, 0.5, 1.0).into());
    handles.vertex_material = materials.add(Color::rgb(0.4, 0.7, 0.6).into());
    let default_wall_material_texture = asset_server.load("sandbox://textures/default.png");
    //handles.wall_material = materials.add(Color::rgb(0.5, 0.5, 1.0).into());
    handles.wall_material = materials.add(StandardMaterial {
        base_color_texture: Some(default_wall_material_texture.clone()),
        unlit: false,
        ..default()
    });

    println!("Initializing site map: {}", sm.site_name);
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.001,
    });

    commands.init_resource::<VerticesManagerData>();

    let mut level_entities: Vec<Entity> = Vec::new();
    let mut level_vertices: Vec<&Vec<Vertex>> = Vec::new();
    for level in &sm.levels {
        // spawn lights
        // todo: calculate bounding box of this level
        let bb = level.calc_bb();
        let make_light_grid = false; // todo: select based on WASM and GPU (or not)
        if make_light_grid {
            // spawn a grid of lights for this level
            let light_spacing = 10.;
            let num_x_lights = ((bb.max_x - bb.min_x) / light_spacing).ceil() as i32;
            let num_y_lights = ((bb.max_y - bb.min_y) / light_spacing).ceil() as i32;
            for x_idx in 0..num_x_lights {
                for y_idx in 0..num_y_lights {
                    let x = bb.min_x + (x_idx as f64) * light_spacing;
                    let y = bb.min_y + (y_idx as f64) * light_spacing;
                    commands
                        .spawn_bundle(PointLightBundle {
                            transform: Transform::from_xyz(x as f32, y as f32, 3.0),
                            point_light: PointLight {
                                intensity: 500.,
                                range: 10.,
                                //shadows_enabled: true,
                                ..default()
                            },
                            ..default()
                        })
                        .insert(SiteMapTag);
                }
            }
        } else {
            // create a single directional light (for machines without GPU)
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
        }

        let vertices = &level.vertices;
        level_vertices.push(vertices);
        level_entities.push(
            commands
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
                .id(),
        );

        // spawn the floor plane
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
    }
    if level_entities.len() == 0 {
        println!("No levels found in site map");
        return;
    }
    commands.insert_resource(SiteMapLevel(level_entities[0]));
    commands.insert_resource(level_vertices[0].clone());

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
    commands.remove_resource::<VerticesManagerData>();
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
                    mesh: meshes.add(wall.mesh(v1, v2)),
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
    mut loading_models: Local<HashMap<Entity, (Model, Handle<Scene>)>>,
    mut spawned: Local<Vec<Entity>>,
) {
    // spawn new models
    {
        // There is a bug(?) in bevy scenes, which causes panic when a scene is despawned
        // immediately after it is spawned.
        // Work around it by checking the `spawned` container BEFORE updating it so that
        // entities are only despawned at the next frame. This also ensures that entities are
        // "fully spawned" before despawning.
        for e in spawned.iter() {
            commands.entity(*e).remove::<DespawnBlocker>();
        }
        spawned.clear();

        for (e, (model, h)) in loading_models.iter() {
            if asset_server.get_load_state(h) == LoadState::Loaded {
                commands
                    .entity(*e)
                    .insert_bundle((model.transform(), GlobalTransform::identity()))
                    .with_children(|parent| {
                        parent.spawn_scene(h.clone());
                    })
                    .insert(Parent(level_entity.0));
                spawned.push(*e);
            }
        }
        for e in spawned.iter() {
            loading_models.remove(e);
        }

        for (e, model) in added_models.iter() {
            let bundle_path =
                String::from("sandbox://") + &model.model_name + &String::from(".glb#Scene0");
            println!(
                "spawning {} at {}, {}",
                &bundle_path, model.x_meters, model.y_meters
            );
            let glb: Handle<Scene> = asset_server.load(&bundle_path);
            commands.entity(e).insert(DespawnBlocker());
            loading_models.insert(e, (model.clone(), glb.clone()));
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
            .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
            .init_resource::<Handles>()
            .init_resource::<MaterialMap>()
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
