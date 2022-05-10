use super::building_map::BuildingMap;
use super::lane::Lane;
use super::level::Level;
use super::measurement::Measurement;
use super::ui_widgets::VisibleWindows;
use super::vertex::Vertex;
use super::wall::Wall;
use bevy::prelude::*;
use bevy::render::camera::{ActiveCamera, Camera3d};
use bevy::ui::Interaction;
use bevy_egui::EguiContext;
use bevy_inspector_egui::plugin::InspectorWindows;
use bevy_inspector_egui::{Inspectable, InspectorPlugin, RegisterInspectable};
use bevy_mod_picking::{DefaultPickingPlugins, PickingBlocker, PickingCamera, PickingCameraBundle};

use std::{
    env,
    fs::{metadata, File},
};

use serde_yaml;

#[derive(Inspectable, Default)]
struct Inspector {
    #[inspectable(deletable = false)]
    active: Option<Editable>,
}

#[derive(Inspectable, Component, Clone)]
pub enum Editable {
    Lane(Lane),
    Measurement(Measurement),
    Vertex(Vertex),
    Wall(Wall),
}

////////////////////////////////////////////////////////
// A few helper structs to use when parsing YAML files
////////////////////////////////////////////////////////

#[derive(Default)]
pub struct Handles {
    pub default_floor_material: Handle<StandardMaterial>,
    pub lane_material: Handle<StandardMaterial>,
    pub measurement_material: Handle<StandardMaterial>,
    pub vertex_mesh: Handle<Mesh>,
    pub vertex_material: Handle<StandardMaterial>,
    pub wall_material: Handle<StandardMaterial>,
}

#[derive(Default)]
pub struct SiteMap {
    site_name: String,
    levels: Vec<Level>,
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

    pub fn from_yaml(doc: &serde_yaml::Value) -> SiteMap {
        let mut sm = SiteMap {
            ..Default::default()
        };

        sm.site_name = doc["name"].as_str().unwrap().to_string();
        for (k, level_yaml) in doc["levels"].as_mapping().unwrap().iter() {
            let name_str = k.as_str().unwrap();
            sm.levels.push(Level::from_yaml(name_str, level_yaml));
        }

        // todo: global alignment via fiducials

        return sm;
    }
}

////////////////////////////////////////////////////////
// A few events to use when requesting to spawn a map
////////////////////////////////////////////////////////

pub struct SpawnSiteMapFilename {
    pub filename: String,
}

pub struct SpawnSiteMapYaml {
    pub yaml_doc: serde_yaml::Value,
}

pub struct SpawnSiteMap {
    pub site_map: SiteMap,
}

pub fn spawn_site_map_filename(
    mut ev_filename: EventReader<SpawnSiteMapFilename>,
    mut ev_yaml: EventWriter<SpawnSiteMapYaml>,
) {
    for ev in ev_filename.iter() {
        let filename = &ev.filename;
        println!("spawn_site_map_filename: : [{}]", filename);
        if !metadata(&filename).is_ok() {
            println!("could not open [{}]", &filename);
            return;
        }
        let file = File::open(&filename).expect("Could not open file");
        let doc: serde_yaml::Value = serde_yaml::from_reader(file).ok().unwrap();
        ev_yaml.send(SpawnSiteMapYaml { yaml_doc: doc });
    }
}

pub fn spawn_site_map_yaml(
    mut ev_yaml: EventReader<SpawnSiteMapYaml>,
    mut ev_site_map: EventWriter<SpawnSiteMap>,
) {
    for ev in ev_yaml.iter() {
        let sm = SiteMap::from_yaml(&ev.yaml_doc);
        ev_site_map.send(SpawnSiteMap { site_map: sm });
    }
}

fn spawn_site_map(
    mut ev_spawn: EventReader<SpawnSiteMap>,
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_query: Query<(Entity, &Handle<Mesh>)>,
    handles: Res<Handles>,
    asset_server: Res<AssetServer>,
) {
    for ev in ev_spawn.iter() {
        let sm = &ev.site_map;

        // first, despawn all existing mesh entities
        println!("despawing all meshes...");
        for entity_mesh in mesh_query.iter() {
            let (entity, _mesh) = entity_mesh;
            commands.entity(entity).despawn_recursive();
        }

        for level in &sm.levels {
            level.spawn(&mut commands, &mut meshes, &handles, &asset_server);
        }

        // todo: use real floor polygons
        commands.spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: 100.0 })),
            material: handles.default_floor_material.clone(),
            transform: Transform {
                rotation: Quat::from_rotation_x(1.57),
                ..Default::default()
            },
            ..Default::default()
        });
    }
}

pub fn initialize_site_map(
    mut commands: Commands,
    mut spawn_filename_writer: EventWriter<SpawnSiteMapFilename>,
) {
    let args: Vec<String> = env::args().collect();
    if args.len() >= 2 {
        spawn_filename_writer.send(SpawnSiteMapFilename {
            filename: args[1].clone(),
        });
    }
    commands
        .spawn()
        .insert(PickingBlocker)
        .insert(Interaction::default());
}

pub fn manage_inspector(
    visible_windows: ResMut<VisibleWindows>,
    mut inspector_windows: ResMut<InspectorWindows>,
) {
    let mut inspector_window_data = inspector_windows.window_data_mut::<Inspector>();
    inspector_window_data.visible = visible_windows.inspector;
}

fn update_picking_cam(
    mut commands: Commands,
    opt_active_camera: Option<Res<ActiveCamera<Camera3d>>>,
    picking_cams: Query<Entity, With<PickingCamera>>,
) {
    let active_camera = match opt_active_camera {
        Some(cam) => cam,
        None => return,
    };
    if active_camera.is_changed() {
        match active_camera.get() {
            Some(active_cam) => {
                // remove all previous picking cameras
                for cam in picking_cams.iter() {
                    commands.entity(cam).remove_bundle::<PickingCameraBundle>();
                }
                commands
                    .entity(active_cam)
                    .insert_bundle(PickingCameraBundle::default());
            }
            None => (),
        }
    }
}

/// Stops picking when egui is in focus.
/// This creates a dummy PickingBlocker and make it "Clicked" whenever egui is in focus.
///
/// Normally bevy_mod_picking automatically stops when
/// a bevy ui node is in focus, but bevy_egui does not use bevy ui node.
fn enable_picking(
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Query<&mut Interaction, With<PickingBlocker>>,
) {
    let egui_ctx = egui_context.ctx_mut();
    let enable = !egui_ctx.wants_pointer_input() && !egui_ctx.wants_keyboard_input();

    let mut blocker = picking_blocker.single_mut();
    if enable {
        *blocker = Interaction::None;
    } else {
        *blocker = Interaction::Clicked;
    }
}

fn maintain_inspected_entities(
    mut inspector: ResMut<Inspector>,
    editables: Query<(&Editable, &Interaction), Changed<Interaction>>,
) {
    let selected = editables.iter().find_map(|(e, i)| match i {
        Interaction::Clicked => Some(e),
        _ => None,
    });
    if let Some(selected) = selected {
        inspector.active = Some(selected.clone())
    }
}

fn init_handles(
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut handles: ResMut<Handles>,
) {
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
}

#[derive(Default)]
pub struct SiteMapPlugin;

impl Plugin for SiteMapPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(DefaultPickingPlugins)
            .add_plugin(InspectorPlugin::<Inspector>::new())
            .register_inspectable::<Lane>()
            .init_resource::<Handles>()
            .add_startup_system(init_handles)
            .add_event::<SpawnSiteMap>()
            .add_event::<SpawnSiteMapFilename>()
            .add_event::<SpawnSiteMapYaml>()
            .add_startup_system(initialize_site_map)
            .add_system(spawn_site_map)
            .add_system(spawn_site_map_yaml)
            .add_system(spawn_site_map_filename)
            .add_system(update_picking_cam)
            .add_system(enable_picking)
            .add_system(maintain_inspected_entities)
            .add_system(manage_inspector);
    }
}
