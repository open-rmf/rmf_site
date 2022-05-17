use crate::level::Level;
use crate::model::Model;
use crate::site_map::{SiteMap, SiteMapLabel};
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

    let pi_2 = 3.1415926 / 2.;
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
                    yaw + pi_2,
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
                    yaw + pi_2,
                ))
                .insert(WarehouseRackTag(generation));

            if idx < num_racks {
                let rack_x = x + ((idx + 1) as f64) * rack_length;
                commands
                    .spawn()
                    .insert(Model::from_xyz_yaw(
                        "horiz_beam1",
                        "OpenRobotics/PalletRackHorBeams",
                        rack_x,
                        y - rack_depth_offset - rack_depth_spacing / 2.,
                        z_offset,
                        yaw + pi_2,
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
                        yaw + pi_2,
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
                        yaw + pi_2,
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
                        yaw + pi_2,
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
