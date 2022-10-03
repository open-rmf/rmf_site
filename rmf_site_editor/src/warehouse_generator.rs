use crate::building_map::BuildingMap;
use crate::despawn::*;
use crate::level::Level;
use crate::light::Light;
use crate::model::Model;
use crate::settings::*;
use crate::spawner::Spawner;
use crate::vertex::Vertex;
use crate::wall::{Wall, WallProperties};
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

// Indicates entites that need to be respawned whenever the warehouse params change.
#[derive(Component)]
struct WarehouseRespawnTag;

struct UiData(Warehouse);

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
    mut spawner: Spawner,
    warehouse: Res<Warehouse>,
    mut vertices: Query<&mut Vertex, With<WarehouseTag>>,
    mut despawn: EventWriter<Despawn>,
    q_respawn_entities: Query<Entity, With<WarehouseRespawnTag>>,
    q_pending_despawns: Query<Entity, (With<PendingDespawn>, With<WarehouseRespawnTag>)>,
    mut need_respawn: Local<bool>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    mut material_map: ResMut<MaterialMap>,
    asset_server: Res<AssetServer>,
    settings: Res<Settings>,
) {
    // despawn previous instance
    if warehouse.is_changed() {
        for e in q_respawn_entities.iter() {
            despawn.send(Despawn(e));
        }
        *need_respawn = true;
        // despawns happen at end of frame, so we need to exit now and wait for the next frame.
        return;
    }

    let pending_despawns = q_pending_despawns.iter().next().is_some();
    // don't spawn new entities if previous ones are still despawning.
    if !*need_respawn || pending_despawns {
        return;
    }

    // previous entities should have despawned by now.
    *need_respawn = false;

    let width = warehouse.area.sqrt();
    let mut vertices: Vec<Mut<Vertex>> = vertices.iter_mut().collect();
    if vertices.len() == 0 {
        return;
    }

    vertices[0].0 = -width / 2.;
    vertices[0].1 = -width / 2.;
    vertices[1].0 = width / 2.;
    vertices[1].1 = -width / 2.;
    vertices[2].0 = width / 2.;
    vertices[2].1 = width / 2.;
    vertices[3].0 = -width / 2.;
    vertices[3].1 = width / 2.;

    let rack_length = 2.3784;
    let roadway_width = 2.0;
    let num_racks = ((width - 2. * roadway_width) / rack_length - 1.) as i32;

    let aisle_width = warehouse.aisle_width;
    let rack_depth = 1.1;
    let aisle_spacing = aisle_width + 2. * rack_depth;
    let num_aisles = (width / aisle_spacing).floor() as i32;
    let rack_depth_spacing = 1.3;
    //let rack_depth_offset = 0.5;

    let vert_stacks = warehouse.height / 2;

    for aisle_idx in 0..num_aisles {
        let aisle_y = (aisle_idx as f64 - (num_aisles as f64 - 1.) / 2.) * aisle_spacing;
        let rack_1_y = aisle_y - aisle_width / 2. + 2. * rack_depth / 2. - rack_depth_spacing;
        let rack_2_y = aisle_y + aisle_width / 2. + 0. * rack_depth / 2. + rack_depth_spacing;
        let rack_start_x = -(width - 2. * roadway_width) / 2. + 1.;
        add_racks(
            &mut spawner,
            "L1",
            rack_start_x,
            rack_1_y,
            0.,
            num_racks,
            vert_stacks,
        );
        add_racks(
            &mut spawner,
            "L1",
            rack_start_x,
            rack_2_y,
            0.,
            num_racks,
            vert_stacks,
        );
        if settings.graphics_quality == GraphicsQuality::Ultra {
            // for now we're a square
            let num_lights_x = num_aisles;
            let light_x_spacing = aisle_spacing;

            for x_idx in 0..num_lights_x {
                let light_x = (x_idx as f64 - (num_lights_x as f64 - 1.) / 2.) * light_x_spacing;
                // spawn some lights
                spawner
                    .spawn_in_level(
                        "L1",
                        Light {
                            x: light_x,
                            y: aisle_y,
                            z_offset: 1.0 + 2.4 * (vert_stacks as f64),
                            intensity: 300.0,
                            range: 10.0,
                        },
                    )
                    .unwrap()
                    .insert(WarehouseRespawnTag);
            }
        }
    }

    // create the floor material if necessary
    // TODO: We should add floor material to level and have site map spawn it. This is needed so
    // that the warehouse will look the same in traffic editor.
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
    spawner: &mut Spawner,
    level: &str,
    x: f64,
    y: f64,
    yaw: f64,
    num_racks: i32,
    num_stacks: i32,
) {
    let rack_length = 2.3784;
    let rack_height = 2.4;

    for idx in 0..(num_racks + 1) {
        for vert_idx in 0..num_stacks {
            let z_offset = (vert_idx as f64) * rack_height;
            spawner
                .spawn_in_level(
                    level,
                    Model::from_xyz_yaw(
                        "vert_beam1",
                        "OpenRobotics/PalletRackVertBeams",
                        x + (idx as f64) * rack_length,
                        y,
                        z_offset,
                        yaw,
                    ),
                )
                .unwrap()
                .insert(WarehouseRespawnTag);

            if idx < num_racks {
                let rack_x = x + (idx as f64) * rack_length;
                spawner
                    .spawn_in_level(
                        level,
                        Model::from_xyz_yaw(
                            "horiz_beam1",
                            "OpenRobotics/PalletRackHorBeams",
                            rack_x,
                            y,
                            z_offset,
                            yaw,
                        ),
                    )
                    .unwrap()
                    .insert(WarehouseRespawnTag);
                let second_shelf_z_offset = 1.0;
                spawner
                    .spawn_in_level(
                        level,
                        Model::from_xyz_yaw(
                            "horiz_beam1",
                            "OpenRobotics/PalletRackHorBeams",
                            rack_x,
                            y,
                            z_offset + second_shelf_z_offset,
                            yaw,
                        ),
                    )
                    .unwrap()
                    .insert(WarehouseRespawnTag);
            }
        }
    }
}

fn on_enter(
    mut commands: Commands,
    mut spawner: Spawner,
    mut sitemap_state: ResMut<State<SiteMapState>>,
) {
    let mut site_map = BuildingMap::default();
    site_map.name = "new site".to_string();
    site_map.levels.insert("L1".to_string(), Level::default());
    spawner.spawn_map(&site_map);
    for i in 0..4 {
        spawner
            .spawn_vertex("L1", Vertex::default())
            .unwrap()
            .insert(WarehouseTag);
        spawner
            .spawn_in_level("L1", Wall(i, (i + 1) % 4, WallProperties::default()))
            .unwrap()
            .insert(WarehouseTag);
    }
    commands.insert_resource(site_map);
    sitemap_state.set(SiteMapState::Enabled).unwrap();
}

fn on_exit(mut commands: Commands, mut sitemap_state: ResMut<State<SiteMapState>>) {
    commands.remove_resource::<BuildingMap>();
    sitemap_state.set(SiteMapState::Disabled).unwrap();
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
        }))
        .init_resource::<MaterialMap>()
        .add_system_set(SystemSet::on_enter(AppState::WarehouseGenerator).with_system(on_enter))
        .add_system_set(SystemSet::on_exit(AppState::WarehouseGenerator).with_system(on_exit))
        .add_system_set(
            SystemSet::on_update(AppState::WarehouseGenerator)
                .with_system(warehouse_ui.before(warehouse_generator))
                .with_system(warehouse_generator.before(SiteMapLabel)),
        );
    }
}
