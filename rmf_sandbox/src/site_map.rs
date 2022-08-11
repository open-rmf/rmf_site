use std::collections::{HashMap, HashSet};

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
            .insert(VertexVisualCue { dagger, halo, body, drag: None })
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
