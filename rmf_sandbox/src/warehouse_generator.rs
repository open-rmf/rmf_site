use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};
use super::level::Level;
use super::model::Model;
use super::ui_widgets::VisibleWindows;
use super::site_map::Handles;
use super::vertex::Vertex;

#[derive(Clone, Default, PartialEq)]
pub struct WarehouseParams {
    pub square_feet: f64,
}

#[derive(Default)]
pub struct WarehouseState {
    pub requested: WarehouseParams,
    pub spawned: WarehouseParams,
}

pub fn warehouse_ui(
    egui_context: &mut EguiContext,
    warehouse_state: &mut WarehouseState,
) {
    egui::SidePanel::left("left").show(egui_context.ctx_mut(), |ui| {
        ui.heading("Warehouse Generator");
        ui.add_space(10.);
        ui.add(
            egui::Slider::new(&mut warehouse_state.requested.square_feet, 200.0..=1000.0)
                .text("Square feet"),
        );
    });
}

fn warehouse_generator(
    mut commands: Commands,
    mut warehouse_state: ResMut<WarehouseState>,
    mut meshes: ResMut<Assets<Mesh>>,
    mesh_query: Query<(Entity, &Handle<Mesh>)>,
    handles: Res<Handles>,
    visible_windows: ResMut<VisibleWindows>,
    asset_server: Res<AssetServer>,
    point_light_query: Query<(Entity, &PointLight)>,
    directional_light_query: Query<(Entity, &DirectionalLight)>,
) {
    if !visible_windows.generator {
        return;
    }
    if warehouse_state.requested != warehouse_state.spawned {
        // first, despawn all existing mesh entities
        for entity_mesh in mesh_query.iter() {
            let (entity, _mesh) = entity_mesh;
            commands.entity(entity).despawn_recursive();
        }
        for entity_light in point_light_query.iter() {
            let (entity, _light) = entity_light;
            commands.entity(entity).despawn_recursive();
        }
        for entity_light in directional_light_query.iter() {
            let (entity, _light) = entity_light;
            commands.entity(entity).despawn_recursive();
        }

        let width = warehouse_state.requested.square_feet.sqrt();
        let mut level = Level::default();
        level.vertices.push(Vertex {
            x_meters: -width / 2.,
            y_meters: -width / 2.,
            ..Default::default()
        });
        level.vertices.push(Vertex {
            x_meters: width / 2.,
            y_meters: -width / 2.,
            ..Default::default()
        });
        level.vertices.push(Vertex {
            x_meters: width / 2.,
            y_meters: width / 2.,
            ..Default::default()
        });
        level.vertices.push(Vertex {
            x_meters: -width / 2.,
            y_meters: width / 2.,
            ..Default::default()
        });
        /*
        level.walls.push(Wall { start: 0, end: 1 });
        level.walls.push(Wall { start: 1, end: 2 });
        level.walls.push(Wall { start: 2, end: 3 });
        level.walls.push(Wall { start: 3, end: 0 });
        */

        add_racks(&mut level, 0., 0., 0., 1);

        level.spawn(&mut commands, &mut meshes, &handles, &asset_server);

        commands.spawn_bundle(PbrBundle {
            mesh: meshes.add(Mesh::from(shape::Plane { size: width as f32 })),
            material: handles.default_floor_material.clone(),
            transform: Transform {
                rotation: Quat::from_rotation_x(1.57),
                ..Default::default()
            },
            ..Default::default()
        });

        let make_light_grid = true; // todo: select based on WASM and GPU (or not)
        if make_light_grid {
            // spawn a grid of lights for this level
            let light_spacing = 10.;
            let num_x_lights = (width / light_spacing).ceil() as i32;
            let num_y_lights = (width / light_spacing).ceil() as i32;
            for x_idx in 0..num_x_lights {
                for y_idx in 0..num_y_lights {
                    let x = -width / 2. + (x_idx as f64) * light_spacing;
                    let y = -width / 2. + (y_idx as f64) * light_spacing;
                    commands.spawn_bundle(PointLightBundle {
                        transform: Transform::from_xyz(x as f32, y as f32, 3.0),
                        point_light: PointLight {
                            intensity: 500.,
                            range: 10.,
                            //shadows_enabled: true,
                            ..default()
                        },
                        ..default()
                    });
                }
            }
        } else {
            // create a single directional light (for machines without GPU)
            commands.spawn_bundle(DirectionalLightBundle {
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
            });
        }

        warehouse_state.spawned = warehouse_state.requested.clone();
    }
}

fn add_racks(
    level: &mut Level,
    x: f64,
    y: f64,
    yaw: f64,
    num_racks: i32,
) {
    let mut num_beam = 0;
    for idx in 0..(num_racks+1) {
        level.models.push(Model::from_xy_yaw(
            "vert_beam1",
            "OpenRobotics/PalletRackVertBeams",
            x + (idx as f64) * 2.3784,
            y,
            yaw + 3.1415926 / 2.
        ));
        /*
        level.models.push(Model::from_xy_yaw(
            "horiz_beam1",
            "OpenRobotics/PalletRackHorBeams",
            x + (idx as f64) * 2.3784,
            y,
            yaw + 3.1415926 / 2.
        ));
        */
    }
}

pub struct WarehouseGeneratorPlugin;

impl Plugin for WarehouseGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WarehouseState {
            requested: WarehouseParams { square_feet: 100. },
            ..Default::default()
        });
        app.add_system(warehouse_generator);
    }
}
