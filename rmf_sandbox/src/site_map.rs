use std::collections::HashMap;

use crate::despawn::{DespawnBlocker, PendingDespawn};
use crate::door::Door;
use crate::floor::Floor;
use crate::interaction::{
    Bobbing, DefaultVisualCue, FloorVisualCue, Hovering, InteractionAssets, LaneVisualCue,
    Selected, Spinning, VertexVisualCue, WallVisualCue,
};
use crate::lane::{Lane, LANE_WIDTH, PASSIVE_LANE_HEIGHT};
use crate::lift::Lift;
use crate::light::Light;
use crate::measurement::Measurement;
use crate::model::Model;
use crate::physical_camera::*;
use crate::settings::*;
use crate::spawner::{SiteMapRoot, VerticesManagers};
use crate::traffic_editor::EditableTag;
use crate::vertex::Vertex;
use crate::{building_map::BuildingMap, wall::Wall};

use bevy::{asset::LoadState, prelude::*};

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

pub struct SiteAssets {
    pub default_floor_material: Handle<StandardMaterial>,
    pub lane_mid_mesh: Handle<Mesh>,
    pub lane_end_mesh: Handle<Mesh>,
    pub passive_lane_material: Handle<StandardMaterial>,
    pub passive_vertex_material: Handle<StandardMaterial>,
    pub hover_material: Handle<StandardMaterial>,
    pub select_material: Handle<StandardMaterial>,
    pub hover_select_material: Handle<StandardMaterial>,
    pub measurement_material: Handle<StandardMaterial>,
    pub vertex_mesh: Handle<Mesh>,
    pub wall_material: Handle<StandardMaterial>,
    pub door_material: Handle<StandardMaterial>,
    pub physical_camera_material: Handle<StandardMaterial>,
}

impl FromWorld for SiteAssets {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let wall_texture = asset_server.load("sandbox://textures/default.png");

        let mut materials = world
            .get_resource_mut::<Assets<StandardMaterial>>()
            .unwrap();
        let passive_lane_material = materials.add(Color::rgb(1.0, 0.5, 0.3).into());
        let select_material = materials.add(Color::rgb(1., 0.3, 1.).into());
        let hover_material = materials.add(Color::rgb(0.3, 1., 1.).into());
        let hover_select_material = materials.add(Color::rgb(1., 0.6, 1.).into());
        let measurement_material = materials.add(Color::rgb_u8(250, 234, 72).into());
        let passive_vertex_material = materials.add(Color::rgb(0.4, 0.7, 0.6).into());
        let wall_material = materials.add(StandardMaterial {
            base_color_texture: Some(wall_texture),
            unlit: false,
            ..default()
        });
        let default_floor_material = materials.add(StandardMaterial {
            base_color: Color::rgb(0.3, 0.3, 0.3).into(),
            perceptual_roughness: 0.5,
            ..default()
        });
        let door_material = materials.add(StandardMaterial {
            base_color: Color::rgba(1., 1., 1., 0.8),
            alpha_mode: AlphaMode::Blend,
            ..default()
        });
        let physical_camera_material = materials.add(Color::rgb(0.6, 0.7, 0.8).into());

        let mut meshes = world.get_resource_mut::<Assets<Mesh>>().unwrap();
        let vertex_mesh = meshes.add(Mesh::from(shape::Capsule {
            radius: 0.15, // TODO(MXG): Make the vertex radius configurable
            rings: 2,
            depth: 0.05,
            latitudes: 8,
            longitudes: 16,
            uv_profile: shape::CapsuleUvProfile::Fixed,
        }));
        let lane_mid_mesh = meshes.add(Mesh::from(shape::Quad::new(Vec2::from([1., 1.]))));
        let lane_end_mesh = meshes.add(Mesh::from(shape::Circle::new(LANE_WIDTH / 2.)));

        Self {
            vertex_mesh,
            default_floor_material,
            lane_mid_mesh,
            lane_end_mesh,
            passive_lane_material,
            hover_material,
            select_material,
            hover_select_material,
            measurement_material,
            passive_vertex_material,
            wall_material,
            door_material,
            physical_camera_material,
        }
    }
}

/// Used to keep track of the entity that represents the current level being rendered by the plugin.
pub struct SiteMapCurrentLevel(pub String);

/// Used to keep track of entities created by the site map system.
#[derive(Component)]
struct SiteMapTag;

#[derive(Default)]
struct LoadingModels(HashMap<Entity, (Model, Handle<Scene>)>);

#[derive(Default)]
struct SpawnedModels(Vec<Entity>);

pub fn init_site_map(sm: Res<BuildingMap>, mut commands: Commands, settings: Res<Settings>) {
    println!("Initializing site map: {}", sm.name);
    commands.insert_resource(AmbientLight {
        color: Color::WHITE,
        brightness: 0.001,
        // brightness: 1.0,
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
                    println!("Inserting light at {x}, {y}");
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
    }
    let current_level = sm.levels.keys().next().unwrap();
    commands.insert_resource(Some(SiteMapCurrentLevel(current_level.clone())));
    commands.insert_resource(LoadingModels::default());
    commands.insert_resource(SpawnedModels::default());
}

fn despawn_site_map(
    mut commands: Commands,
    site_map_entities: Query<Entity, With<SiteMapTag>>,
    map_root: Query<Entity, With<SiteMapRoot>>,
    mut level: ResMut<Option<SiteMapCurrentLevel>>,
) {
    // removing this causes bevy to panic, instead just replace it with the default.
    commands.init_resource::<AmbientLight>();

    println!("Despawn all entites");
    for entity in site_map_entities.iter() {
        commands.entity(entity).despawn_recursive();
    }

    *level = None;
    for e in map_root.iter() {
        commands.entity(e).insert(PendingDespawn);
    }
}

fn update_floor(
    mut commands: Commands,
    q_floors: Query<Entity, Added<Floor>>,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<SiteAssets>,
) {
    for e in q_floors.iter() {
        // spawn the floor plane
        commands
            .entity(e)
            .insert_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
                material: handles.default_floor_material.clone(),
                transform: Transform {
                    rotation: Quat::from_rotation_x(std::f32::consts::PI / 2.),
                    ..default()
                },
                ..default()
            })
            .insert(FloorVisualCue)
            .insert(Hovering::default())
            .insert(Selected::default());
    }
}

fn update_vertices(
    mut commands: Commands,
    handles: Res<SiteAssets>,
    added_vertices: Query<(Entity, &Vertex), Added<Vertex>>,
    mut changed_vertices: Query<(&Vertex, &mut Transform), Changed<Vertex>>,
    interaction_assets: Res<InteractionAssets>,
) {
    // spawn new vertices
    for (e, v) in added_vertices.iter() {
        let mut commands = commands.entity(e);
        commands.insert_bundle(SpatialBundle {
            transform: v.transform(),
            ..default()
        });

        let (dagger, halo, body) = commands.add_children(|parent| {
            let dagger = parent
                .spawn_bundle(PbrBundle {
                    material: interaction_assets.dagger_material.clone(),
                    mesh: interaction_assets.dagger_mesh.clone(),
                    visibility: Visibility { is_visible: false },
                    ..default()
                })
                .insert(Bobbing::default())
                .insert(Spinning::default())
                .insert(EditableTag::Ignore)
                .id();

            let halo = parent
                .spawn_bundle(PbrBundle {
                    // Have the halo fit nicely around a vertex
                    transform: Transform::from_scale([0.2, 0.2, 1.].into()),
                    material: interaction_assets.halo_material.clone(),
                    mesh: interaction_assets.halo_mesh.clone(),
                    visibility: Visibility { is_visible: false },
                    ..default()
                })
                .insert(Spinning::default())
                .insert(EditableTag::Ignore)
                .id();

            let body = parent
                .spawn_bundle(PbrBundle {
                    mesh: handles.vertex_mesh.clone(),
                    material: handles.passive_vertex_material.clone(),
                    transform: Transform::from_rotation(Quat::from_rotation_x(90_f32.to_radians())),
                    ..default()
                })
                .id();

            (dagger, halo, body)
        });

        commands
            .insert(VertexVisualCue { dagger, halo, body })
            .insert(Hovering::default())
            .insert(Selected::default());
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
        println!("Updating light {e:?}");
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

#[derive(Component, Debug, Clone, Copy)]
pub struct LanePieces {
    pub segments: [Entity; 3],
}

impl LanePieces {
    pub fn start(&self) -> Entity {
        self.segments[0]
    }

    pub fn mid(&self) -> Entity {
        self.segments[1]
    }

    pub fn end(&self) -> Entity {
        self.segments[2]
    }
}

fn update_lanes(
    mut commands: Commands,
    handles: Res<SiteAssets>,
    vertices_mgrs: Res<VerticesManagers>,
    level: Res<Option<SiteMapCurrentLevel>>,
    vertices: Query<(&Vertex, ChangeTrackers<Vertex>)>,
    mut lanes: Query<(Entity, &Lane, ChangeTrackers<Lane>, Option<&LanePieces>)>,
    mut transforms: Query<&mut Transform>,
) {
    let level = match level.as_ref() {
        Some(level) => level,
        None => {
            return;
        }
    };
    // spawn new lanes
    for (e, lane, change, pieces) in lanes.iter_mut() {
        let v1_entity = vertices_mgrs.0[&level.0].id_to_entity(lane.0).unwrap();
        let (v1, v1_change) = vertices.get(v1_entity).unwrap();
        let v2_entity = vertices_mgrs.0[&level.0].id_to_entity(lane.1).unwrap();
        let (v2, v2_change) = vertices.get(v2_entity).unwrap();

        if let Some(pieces) = pieces {
            if change.is_changed() || v1_change.is_changed() || v2_change.is_changed() {
                if let Some(mut tf) = transforms.get_mut(pieces.start()).ok() {
                    *tf = v1.transform();
                }
                if let Some(mut tf) = transforms.get_mut(pieces.mid()).ok() {
                    *tf = lane.transform(v1, v2);
                }
                if let Some(mut tf) = transforms.get_mut(pieces.end()).ok() {
                    *tf = v2.transform();
                }
            }
        } else {
            let mut commands = commands.entity(e);
            let (start, mid, end) = commands.add_children(|parent| {
                let start = parent
                    .spawn_bundle(PbrBundle {
                        mesh: handles.lane_end_mesh.clone(),
                        material: handles.passive_lane_material.clone(),
                        transform: v1.transform(),
                        ..default()
                    })
                    .id();

                let mid = parent
                    .spawn_bundle(PbrBundle {
                        mesh: handles.lane_mid_mesh.clone(),
                        material: handles.passive_lane_material.clone(),
                        transform: lane.transform(v1, v2),
                        ..default()
                    })
                    .id();

                let end = parent
                    .spawn_bundle(PbrBundle {
                        mesh: handles.lane_end_mesh.clone(),
                        material: handles.passive_lane_material.clone(),
                        transform: v2.transform(),
                        ..default()
                    })
                    .id();

                (start, mid, end)
            });

            commands
                .insert(LanePieces {
                    segments: [start, mid, end],
                })
                .insert(LaneVisualCue::default())
                .insert(Hovering::default())
                .insert(Selected::default())
                .insert_bundle(SpatialBundle {
                    transform: Transform::from_translation([0., 0., PASSIVE_LANE_HEIGHT].into()),
                    ..default()
                });
        }
    }
}

fn update_measurements(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<SiteAssets>,
    level: Res<Option<SiteMapCurrentLevel>>,
    vertices_mgrs: Res<VerticesManagers>,
    vertices: Query<(&Vertex, ChangeTrackers<Vertex>)>,
    mut measurements: Query<(
        Entity,
        &Measurement,
        ChangeTrackers<Measurement>,
        Option<&mut Transform>,
    )>,
) {
    let level = match level.as_ref() {
        Some(level) => level,
        None => {
            return;
        }
    };
    // spawn new measurements
    for (e, measurement, change, t) in measurements.iter_mut() {
        let v1_entity = vertices_mgrs.0[&level.0]
            .id_to_entity(measurement.0)
            .unwrap();
        let (v1, v1_change) = vertices.get(v1_entity).unwrap();
        let v2_entity = vertices_mgrs.0[&level.0]
            .id_to_entity(measurement.1)
            .unwrap();
        let (v2, v2_change) = vertices.get(v2_entity).unwrap();

        if change.is_added() {
            commands.entity(e).insert_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Quad::new(Vec2::from([1., 1.])))),
                material: handles.measurement_material.clone(),
                transform: measurement.transform(v1, v2),
                ..Default::default()
            });
        } else if change.is_changed() || v1_change.is_changed() || v2_change.is_changed() {
            *t.unwrap() = measurement.transform(v1, v2);
        }
    }
}

fn update_walls(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<SiteAssets>,
    level: Res<Option<SiteMapCurrentLevel>>,
    vertices_mgrs: Res<VerticesManagers>,
    vertices: Query<(&Vertex, ChangeTrackers<Vertex>)>,
    mut walls: Query<(Entity, &Wall, ChangeTrackers<Wall>)>,
) {
    let level = match level.as_ref() {
        Some(level) => level,
        None => {
            return;
        }
    };
    // spawn new walls
    for (e, wall, change) in walls.iter_mut() {
        let v1_entity = vertices_mgrs.0[&level.0].id_to_entity(wall.0).unwrap();
        let (v1, v1_change) = vertices.get(v1_entity).unwrap();
        let v2_entity = vertices_mgrs.0[&level.0].id_to_entity(wall.1).unwrap();
        let (v2, v2_change) = vertices.get(v2_entity).unwrap();

        if change.is_changed() || v1_change.is_changed() || v2_change.is_changed() {
            commands
                .entity(e)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(wall.mesh(v1, v2)),
                    material: handles.wall_material.clone(),
                    transform: wall.transform(v1, v2),
                    ..default()
                })
                .insert(WallVisualCue)
                .insert(Hovering::default())
                .insert(Selected::default());
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
            .insert(DespawnBlocker)
            .insert(ModelCurrentScene(model.model_name.clone()))
            .insert(Hovering::default())
            .insert(Selected::default())
            .insert(DefaultVisualCue);
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
                .insert_bundle(SpatialBundle {
                    transform: model.transform(),
                    ..default()
                })
                .with_children(|parent| {
                    parent.spawn_bundle(SceneBundle {
                        scene: h.clone(),
                        ..default()
                    });
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

fn update_doors(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<SiteAssets>,
    level: Res<Option<SiteMapCurrentLevel>>,
    vertices_mgrs: Res<VerticesManagers>,
    vertices: Query<(&Vertex, ChangeTrackers<Vertex>)>,
    mut q_doors: Query<(Entity, &Door, Option<&mut Transform>, ChangeTrackers<Door>)>,
) {
    let level = match level.as_ref() {
        Some(level) => level,
        None => {
            return;
        }
    };
    for (e, door, t, door_changed) in q_doors.iter_mut() {
        let v1_entity = vertices_mgrs.0[&level.0].id_to_entity(door.0).unwrap();
        let (v1, v1_change) = vertices.get(v1_entity).unwrap();
        let v2_entity = vertices_mgrs.0[&level.0].id_to_entity(door.1).unwrap();
        let (v2, v2_change) = vertices.get(v2_entity).unwrap();

        if !door_changed.is_changed() && !v1_change.is_changed() && !v2_change.is_changed() {
            continue;
        }

        let p1 = Vec3::new(v1.0 as f32, v1.1 as f32, 0.);
        let p2 = Vec3::new(v2.0 as f32, v2.1 as f32, 0.);
        let dist = p1.distance(p2);
        let mid = (p1 + p2) / 2.;
        let rot = f32::atan2(p2.y - p1.y, p2.x - p1.x);
        // width and height is not available from the building file so we use a fixed height.
        let width = 0.1 as f32;
        let height = 4. as f32;

        let transform = Transform {
            translation: mid,
            rotation: Quat::from_rotation_z(rot),
            scale: Vec3::new(dist, width, height),
        };

        if door_changed.is_added() {
            commands
                .entity(e)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Box::new(1., 1., 1.))),
                    material: handles.door_material.clone(),
                    transform,
                    ..default()
                })
                .insert(Hovering::default())
                .insert(Selected::default())
                .insert(DefaultVisualCue);
        }

        if door_changed.is_changed() {
            if let Some(mut t) = t {
                *t = transform;
            }
        }
    }
}

fn update_lifts(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<SiteAssets>,
    mut q_lifts: Query<
        (Entity, &Lift, Option<&mut Transform>, ChangeTrackers<Lift>),
        Changed<Lift>,
    >,
) {
    for (e, lift, t, lift_changes) in q_lifts.iter_mut() {
        let center = Vec3::new(lift.x as f32, lift.y as f32, 0.);
        // height is not available from the building file so we use a fixed height.
        let height = 4. as f32;

        let transform = Transform {
            translation: center,
            rotation: Quat::from_rotation_z(lift.yaw as f32),
            scale: Vec3::new(lift.width as f32, lift.depth as f32, height),
        };

        if lift_changes.is_added() {
            commands
                .entity(e)
                .insert_bundle(PbrBundle {
                    mesh: meshes.add(Mesh::from(shape::Box::new(1., 1., 1.))),
                    material: handles.door_material.clone(),
                    transform,
                    ..default()
                })
                .insert(Hovering::default())
                .insert(Selected::default())
                .insert(DefaultVisualCue);
        }

        if lift_changes.is_changed() {
            if let Some(mut t) = t {
                *t = transform;
            }
        }
    }
}

fn update_cameras(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    handles: Res<SiteAssets>,
    mut q_physical_cameras: Query<
        (
            Entity,
            &PhysicalCamera,
            Option<&mut Transform>,
            ChangeTrackers<PhysicalCamera>,
        ),
        Changed<PhysicalCamera>,
    >,
) {
    for (e, physical_camera, t, changes) in q_physical_cameras.iter_mut() {
        if changes.is_added() {
            commands.entity(e).insert_bundle(PbrBundle {
                mesh: meshes.add(Mesh::from(Pyramid::new(1., 1.))),
                material: handles.physical_camera_material.clone(),
                transform: physical_camera.transform(),
                ..default()
            });
        }

        if changes.is_changed() {
            if let Some(mut t) = t {
                *t = physical_camera.transform();
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
            .init_resource::<SiteAssets>()
            .init_resource::<Option<SiteMapCurrentLevel>>()
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
                    .with_system(update_floor)
                    .with_system(update_vertices.after(init_site_map))
                    .with_system(update_lanes.after(update_vertices))
                    .with_system(update_walls.after(update_vertices))
                    .with_system(update_measurements.after(update_vertices))
                    .with_system(update_lights.after(init_site_map))
                    .with_system(update_models.after(init_site_map))
                    .with_system(update_doors.after(update_vertices))
                    .with_system(update_lifts.after(init_site_map))
                    .with_system(update_cameras.after(init_site_map)),
            );
    }
}
