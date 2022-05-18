use crate::level::Level;
use crate::model::Model;
use crate::site_map::{MaterialMap, SiteMap, SiteMapLabel};
use crate::vertex::Vertex;
use crate::wall::Wall;
use crate::AppState;
use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

#[derive(Default, Clone)]
struct Warehouse {
    pub area: f64,
    pub height: i32,
    pub aisle_width: f64,
}

#[derive(Component)]
struct WarehouseTag;

#[derive(Component)]
struct WarehouseRackTag(usize);

struct UiData(Warehouse);

fn warehouse_ui(
    mut egui_context: ResMut<EguiContext>,
    mut ui_data: ResMut<UiData>,
    mut warehouse: ResMut<Warehouse>,
) {
    let warehouse_request = &mut ui_data.0;

    egui::SidePanel::left("left").show(egui_context.ctx_mut(), |ui| {
        ui.heading("Warehouse Generator");
        ui.add_space(10.);
        if ui
            .add(egui::Slider::new(&mut warehouse_request.area, 400.0..=1000.0).text("Area (m^2)"))
            .changed()
        {
            *warehouse = warehouse_request.clone();
        }
        if ui
            .add(
                egui::Slider::new(&mut warehouse_request.aisle_width, 2.0..=8.0)
                    .text("Aisle width (m)"),
            )
            .changed()
        {
            *warehouse = warehouse_request.clone();
        };
        if ui
            .add(
                egui::Slider::new(&mut warehouse_request.height, 2..=6)
                    .text("Shelf height (m)")
                    .step_by(2.),
            )
            .changed()
        {
            *warehouse = warehouse_request.clone();
        };
    });
}

fn warehouse_generator(
    mut commands: Commands,
    warehouse: Res<Warehouse>,
    mut vertices: Query<&mut Vertex, With<WarehouseTag>>,
    warehouse_racks: Query<(Entity, &WarehouseRackTag), With<WarehouseRackTag>>,
    mut generation: Local<usize>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut material_map: ResMut<MaterialMap>,
    // mesh_query: Query<(Entity, &Handle<Mesh>)>,
    //handles: Res<Handles>,
    asset_server: Res<AssetServer>,
    //point_light_query: Query<(Entity, &PointLight)>,
    //directional_light_query: Query<(Entity, &DirectionalLight)>,
) {
    if !warehouse.is_changed() {
        return;
    }

    let width = warehouse.area.sqrt();
    let mut vertices: Vec<Mut<Vertex>> = vertices.iter_mut().collect();

    vertices[0].x_meters = -width / 2.;
    vertices[0].y_meters = -width / 2.;
    vertices[1].x_meters = width / 2.;
    vertices[1].y_meters = -width / 2.;
    vertices[2].x_meters = width / 2.;
    vertices[2].y_meters = width / 2.;
    vertices[3].x_meters = -width / 2.;
    vertices[3].y_meters = width / 2.;

    let rack_length = 2.3784;
    let num_racks = (width / rack_length - 1.) as i32;

    let aisle_width = warehouse.aisle_width;
    let rack_depth = 1.3;
    let aisle_spacing = aisle_width + 2. * rack_depth;
    let num_aisles = (width / aisle_spacing).floor() as i32;

    let vert_stacks = warehouse.height / 2;

    // clear all previous racks.
    *generation += 1;
    for (e, tag) in warehouse_racks.iter() {
        if tag.0 < *generation {
            commands.entity(e).despawn_recursive();
            /*
            let make_light_grid = true; // todo: select based on WASM and GPU (or not)
            if make_light_grid {
                // spawn a grid of lights for this level
                let light_spacing = 10.;
                let num_x_lights = (width / light_spacing).ceil() as i32;
                let num_y_lights = (width / light_spacing).ceil() as i32;
                let light_height = (warehouse_state.requested.height as f32) * 1.3 + 1.5;
                let light_range = light_height * 3.0;
                for x_idx in 0..num_x_lights {
                    for y_idx in 0..num_y_lights {
                        let x = (x_idx as f64 - (num_x_lights as f64 - 1.) / 2.) * light_spacing;
                        let y = (y_idx as f64 - (num_y_lights as f64 - 1.) / 2.) * light_spacing;
                        commands.spawn_bundle(PointLightBundle {
                            transform: Transform::from_xyz(x as f32, y as f32, light_height),
                            point_light: PointLight {
                                intensity: 2000.,
                                range: light_range,
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
            */
        }
    }

    for aisle_idx in 0..num_aisles {
        let y = (aisle_idx as f64 - (num_aisles as f64 - 1.) / 2.) * aisle_spacing;
        add_racks(
            &mut commands,
            *generation,
            -width / 2. + 1.,
            y,
            0.,
            num_racks,
            vert_stacks,
        );
    }

    // create the floor material if necessary
    if !material_map.materials.contains_key("concrete_floor") {
        let albedo = asset_server.load("sandbox://textures/concrete_albedo_1024.png");
        let roughness = asset_server.load("sandbox://textures/concrete_roughness_1024.png");
        let concrete_floor_handle = materials.add(StandardMaterial {
            base_color_texture: Some(albedo.clone()),
            perceptual_roughness: 0.3,
            metallic_roughness_texture: Some(roughness.clone()),
            ..default()
        });
        material_map
            .materials
            .insert(String::from("concrete_floor"), concrete_floor_handle);
    }

    commands.spawn_bundle(PbrBundle {
        mesh: meshes.add(Mesh::from(shape::Plane { size: width as f32 })),
        material: material_map
            .materials
            .get("concrete_floor")
            .unwrap()
            .clone(),
        transform: Transform {
            rotation: Quat::from_rotation_x(1.5707963),
            ..Default::default()
        },
        ..Default::default()
    });
}

fn add_racks(
    commands: &mut Commands,
    generation: usize,
    x: f64,
    y: f64,
    yaw: f64,
    num_racks: i32,
    num_stacks: i32,
) {
    let rack_depth_spacing = 1.3;
    let rack_depth_offset = 0.5;
    let rack_length = 2.3784;
    let rack_height = 2.4;

    for idx in 0..(num_racks + 1) {
        for vert_idx in 0..num_stacks {
            let z_offset = (vert_idx as f64) * rack_height;
            commands
                .spawn()
                .insert(Model::from_xyz_yaw(
                    "vert_beam1",
                    "OpenRobotics/PalletRackVertBeams",
                    x + (idx as f64) * rack_length,
                    y - rack_depth_offset - rack_depth_spacing / 2.,
                    z_offset,
                    yaw,
                ))
                .insert(WarehouseRackTag(generation));
            commands
                .spawn()
                .insert(Model::from_xyz_yaw(
                    "vert_beam1",
                    "OpenRobotics/PalletRackVertBeams",
                    x + (idx as f64) * rack_length,
                    y - rack_depth_offset + rack_depth_spacing / 2.,
                    z_offset,
                    yaw,
                ))
                .insert(WarehouseRackTag(generation));

            if idx < num_racks {
                let rack_x = x + (idx as f64) * rack_length;
                commands
                    .spawn()
                    .insert(Model::from_xyz_yaw(
                        "horiz_beam1",
                        "OpenRobotics/PalletRackHorBeams",
                        rack_x,
                        y - rack_depth_offset - rack_depth_spacing / 2.,
                        z_offset,
                        yaw,
                    ))
                    .insert(WarehouseRackTag(generation));
                commands
                    .spawn()
                    .insert(Model::from_xyz_yaw(
                        "horiz_beam1",
                        "OpenRobotics/PalletRackHorBeams",
                        rack_x,
                        y - rack_depth_offset + rack_depth_spacing / 2.,
                        z_offset,
                        yaw,
                    ))
                    .insert(WarehouseRackTag(generation));
                let second_shelf_z_offset = 1.0;
                commands
                    .spawn()
                    .insert(Model::from_xyz_yaw(
                        "horiz_beam1",
                        "OpenRobotics/PalletRackHorBeams",
                        rack_x,
                        y - rack_depth_offset - rack_depth_spacing / 2.,
                        z_offset + second_shelf_z_offset,
                        yaw,
                    ))
                    .insert(WarehouseRackTag(generation));
                commands
                    .spawn()
                    .insert(Model::from_xyz_yaw(
                        "horiz_beam1",
                        "OpenRobotics/PalletRackHorBeams",
                        rack_x,
                        y - rack_depth_offset + rack_depth_spacing / 2.,
                        z_offset + second_shelf_z_offset,
                        yaw,
                    ))
                    .insert(WarehouseRackTag(generation));
            }
        }
    }
}

fn on_enter(mut commands: Commands) {
    let mut site_map = SiteMap::default();
    site_map.site_name = "new site".to_string();
    site_map.levels.push(Level::default());
    for i in 0..4 {
        commands
            .spawn()
            .insert(Vertex::default())
            .insert(WarehouseTag);
        commands
            .spawn()
            .insert(Wall {
                start: i,
                end: (i + 1) % 4,
                ..default()
            })
            .insert(WarehouseTag);
    }
    commands.insert_resource(site_map);
}

fn on_exit(mut commands: Commands) {
    commands.remove_resource::<SiteMap>();
}

pub struct WarehouseGeneratorPlugin;

impl Plugin for WarehouseGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(Warehouse {
            area: 400.,
            height: 2,
            aisle_width: 5.,
            ..Default::default()
        })
        .insert_resource(UiData(Warehouse {
            area: 400.,
            height: 2,
            aisle_width: 5.,
            ..Default::default()
        }));
        app.add_system_set(SystemSet::on_enter(AppState::WarehouseGenerator).with_system(on_enter));
        app.add_system_set(
            SystemSet::on_update(AppState::WarehouseGenerator)
                .with_system(warehouse_ui)
                // FIXME: Since spawning of the actual meshes is done by SiteMapPlugin and bevy commands
                // are ran at the end of a stage, it is possible for entities to both despawn and spawn
                // "at the same time", this will cause bevy to panic.
                // The exclusive system is a super hacky workaround to make it not panic, we should
                // look at a proper solution that avoids race condition.
                .with_system(warehouse_generator.exclusive_system().before(SiteMapLabel)),
        );
        app.add_system_set(SystemSet::on_exit(AppState::WarehouseGenerator).with_system(on_exit));
    }
}
