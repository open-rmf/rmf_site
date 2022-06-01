use std::collections::HashMap;

use crate::despawn::DespawnBlocker;
use crate::lane::Lane;
use crate::light::Light;
use crate::measurement::Measurement;
use crate::model::Model;
use crate::settings::*;
use crate::spawner::VerticesManagers;
use crate::vertex::Vertex;
use crate::{building_map::BuildingMap, wall::Wall};
use bevy::asset::LoadState;
use bevy::prelude::*;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum SiteMapState {
    Enabled,
    Disabled,
}

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

/// Used to keep track of the entity that represents the current level being rendered by the plugin.
struct SiteMapCurrentLevel(String);

/// Used to keep track of entities created by the site map system.
#[derive(Component)]
struct SiteMapTag;

#[derive(Default)]
struct LoadingModels(HashMap<Entity, (Model, Handle<Scene>)>);

#[derive(Default)]
struct SpawnedModels(Vec<Entity>);

fn init_site_map(
    sm: Res<BuildingMap>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
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

    println!("Initializing site map: {}", sm.name);
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.001,
    });

    for level in sm.levels.values() {
        // spawn lights
        let bb = level.calc_bb();
        if settings.graphics_quality == GraphicsQuality::Ultra {
            // spawn a grid of lights for this level
            // todo: make UI controls for light spacing, intensity, range, shadows
            let light_spacing = 5.;
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
                                intensity: 300.,
                                range: 7.,
                                //shadows_enabled: true,
                                ..default()
                            },
                            ..default()
                        })
                        .insert(SiteMapTag);
                }
            }
        }

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
    let current_level = sm.levels.keys().next().unwrap();
    commands.insert_resource(SiteMapCurrentLevel(current_level.clone()));

    commands.insert_resource(handles);
    commands.insert_resource(LoadingModels::default());
    commands.insert_resource(SpawnedModels::default());

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
    commands.remove_resource::<SiteMapCurrentLevel>();
}

fn update_vertices(
    mut commands: Commands,
    handles: Res<Handles>,
    added_vertices: Query<(Entity, &Vertex), Added<Vertex>>,
    mut changed_vertices: Query<(&Vertex, &mut Transform), Changed<Vertex>>,
) {
    // spawn new vertices
    for (e, v) in added_vertices.iter() {
        commands.entity(e).insert_bundle(PbrBundle {
            mesh: handles.vertex_mesh.clone(),
            material: handles.vertex_material.clone(),
            transform: v.transform(),
            ..Default::default()
        });
    }
    // update changed vertices
    for (v, mut t) in changed_vertices.iter_mut() {
        *t = v.transform();
    }
}

fn update_lights(
    mut commands: Commands,
    added_lights: Query<(Entity, &Light), Added<Light>>,
    mut changed_lights: Query<(&Light, &mut Transform), Changed<Light>>,
) {
    // spawn new lights
    for (e, light) in added_lights.iter() {
        commands.entity(e).insert_bundle(PointLightBundle {
            transform: light.transform(),
            point_light: PointLight {
                intensity: light.intensity as f32,
                range: light.range as f32,
                ..default()
            },
            ..default()
        });
    }
    // update changed lights
    for (light, mut t) in changed_lights.iter_mut() {
        *t = light.transform();
    }
}

fn update_lanes(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<Handles>,
    vertices_mgrs: Res<VerticesManagers>,
    level: Res<SiteMapCurrentLevel>,
    vertices: Query<(&Vertex, ChangeTrackers<Vertex>)>,
    mut lanes: Query<(Entity, &Lane, ChangeTrackers<Lane>, Option<&mut Transform>)>,
) {
    // spawn new lanes
    for (e, lane, change, t) in lanes.iter_mut() {
        let v1_entity = vertices_mgrs.0[&level.0].get(lane.0).unwrap();
        let (v1, v1_change) = vertices.get(v1_entity).unwrap();
        let v2_entity = vertices_mgrs.0[&level.0].get(lane.1).unwrap();
        let (v2, v2_change) = vertices.get(v2_entity).unwrap();

        if change.is_added() {
            commands
                .entity(e)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::from([1., 1.])))),
                    material: handles.lane_material.clone(),
                    transform: lane.transform(v1, v2),
                    ..Default::default()
                })
                .insert(lane.clone());
        } else if change.is_changed() || v1_change.is_changed() || v2_change.is_changed() {
            *t.unwrap() = lane.transform(v1, v2);
        }
    }
}

fn update_measurements(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<Handles>,
    level: Res<SiteMapCurrentLevel>,
    vertices_mgrs: Res<VerticesManagers>,
    vertices: Query<(&Vertex, ChangeTrackers<Vertex>)>,
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
        let v1_entity = vertices_mgrs.0[&level.0].get(measurement.0).unwrap();
        let (v1, v1_change) = vertices.get(v1_entity).unwrap();
        let v2_entity = vertices_mgrs.0[&level.0].get(measurement.1).unwrap();
        let (v2, v2_change) = vertices.get(v2_entity).unwrap();

        if change.is_added() {
            commands
                .entity(e)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::from([1., 1.])))),
                    material: handles.measurement_material.clone(),
                    transform: measurement.transform(v1, v2),
                    ..Default::default()
                })
                .insert(measurement.clone());
        } else if change.is_changed() || v1_change.is_changed() || v2_change.is_changed() {
            *t.unwrap() = measurement.transform(v1, v2);
        }
    }
}

fn update_walls(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<Handles>,
    level: Res<SiteMapCurrentLevel>,
    vertices_mgrs: Res<VerticesManagers>,
    vertices: Query<(&Vertex, ChangeTrackers<Vertex>)>,
    mut walls: Query<(Entity, &Wall, ChangeTrackers<Wall>)>,
) {
    // spawn new walls
    for (e, wall, change) in walls.iter_mut() {
        let v1_entity = vertices_mgrs.0[&level.0].get(wall.0).unwrap();
        let (v1, v1_change) = vertices.get(v1_entity).unwrap();
        let v2_entity = vertices_mgrs.0[&level.0].get(wall.1).unwrap();
        let (v2, v2_change) = vertices.get(v2_entity).unwrap();

        if change.is_added()
            || change.is_changed()
            || v1_change.is_changed()
            || v2_change.is_changed()
        {
            commands
                .entity(e)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(wall.mesh(v1, v2)),
                    material: handles.wall_material.clone(),
                    transform: wall.transform(v1, v2),
                    ..Default::default()
                })
                .insert(wall.clone());
        }
    }
}

#[derive(Component)]
struct ModelCurrentScene(String);

fn update_models(
    mut commands: Commands,
    added_models: Query<(Entity, &Model), Added<Model>>,
    mut changed_models: Query<(Entity, &Model, &mut Transform), (Changed<Model>, With<Model>)>,
    asset_server: Res<AssetServer>,
    mut loading_models: ResMut<LoadingModels>,
    mut spawned_models: ResMut<SpawnedModels>,
    q_current_scene: Query<&ModelCurrentScene>,
) {
    fn spawn_model(
        e: Entity,
        model: &Model,
        asset_server: &AssetServer,
        commands: &mut Commands,
        loading_models: &mut LoadingModels,
    ) {
        let bundle_path =
            String::from("sandbox://") + &model.model_name + &String::from(".glb#Scene0");
        let glb: Handle<Scene> = asset_server.load(&bundle_path);
        commands
            .entity(e)
            .insert(DespawnBlocker())
            .insert(ModelCurrentScene(model.model_name.clone()));
        loading_models.0.insert(e, (model.clone(), glb.clone()));
    }

    // spawn new models

    // There is a bug(?) in bevy scenes, which causes panic when a scene is despawned
    // immediately after it is spawned.
    // Work around it by checking the `spawned` container BEFORE updating it so that
    // entities are only despawned at the next frame. This also ensures that entities are
    // "fully spawned" before despawning.
    for e in spawned_models.0.iter() {
        commands.entity(*e).remove::<DespawnBlocker>();
    }
    spawned_models.0.clear();

    for (e, (model, h)) in loading_models.0.iter() {
        if asset_server.get_load_state(h) == LoadState::Loaded {
            commands
                .entity(*e)
                .insert_bundle((model.transform(), GlobalTransform::identity()))
                .with_children(|parent| {
                    parent.spawn_scene(h.clone());
                });
            spawned_models.0.push(*e);
        }
    }
    for e in spawned_models.0.iter() {
        loading_models.0.remove(e);
    }

    for (e, model) in added_models.iter() {
        spawn_model(e, model, &asset_server, &mut commands, &mut loading_models);
    }
    // update changed models
    for (e, model, mut t) in changed_models.iter_mut() {
        *t = model.transform();
        if let Ok(current_scene) = q_current_scene.get(e) {
            if current_scene.0 != model.model_name {
                // is this safe since we are also doing the spawning?
                // aside from possibly despawning children created by other plugins.
                commands.entity(e).despawn_descendants();
                spawn_model(e, model, &asset_server, &mut commands, &mut loading_models);
            }
        }
    }
}

#[derive(Default)]
pub struct SiteMapPlugin;

impl Plugin for SiteMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(SiteMapState::Disabled)
            .init_resource::<Vec<Vertex>>()
            .insert_resource(ClearColor(Color::rgb(0.0, 0.0, 0.0)))
            .init_resource::<Handles>()
            .init_resource::<MaterialMap>()
            .add_system_set(
                SystemSet::on_enter(SiteMapState::Enabled)
                    .label(SiteMapLabel)
                    .with_system(init_site_map),
            )
            .add_system_set(
                SystemSet::on_exit(SiteMapState::Enabled)
                    .label(SiteMapLabel)
                    .with_system(despawn_site_map),
            )
            .add_system_set(
                SystemSet::on_update(SiteMapState::Enabled)
                    .label(SiteMapLabel)
                    .with_system(update_vertices.after(init_site_map))
                    .with_system(update_lanes.after(update_vertices))
                    .with_system(update_walls.after(update_vertices))
                    .with_system(update_measurements.after(update_vertices))
                    .with_system(update_lights.after(init_site_map))
                    .with_system(update_models.after(init_site_map)),
            );
    }
}
