use crate::level::Level;
use crate::site_map::{SiteMap, SiteMapLabel};
use crate::vertex::Vertex;
use crate::wall::Wall;
use crate::AppState;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

#[derive(Default)]
struct Warehouse {
    square_feet: f64,
}

#[derive(Component)]
struct WarehouseTag;

fn egui_ui(mut egui_context: ResMut<EguiContext>, mut warehouse: ResMut<Warehouse>) {
    egui::SidePanel::left("left").show(egui_context.ctx_mut(), |ui| {
        ui.heading("Warehouse Generator");
        ui.add_space(10.);
        ui.add(egui::Slider::new(&mut warehouse.square_feet, 100.0..=1000.0).text("Square feet"));
    });
}

fn warehouse_generator(
    warehouse: Res<Warehouse>,
    mut vertices: Query<&mut Vertex, With<WarehouseTag>>,
) {
    if !warehouse.is_changed() {
        return;
    }

    let width = warehouse.square_feet.sqrt();
    let mut vertices: Vec<Mut<Vertex>> = vertices.iter_mut().collect();

    vertices[0].x_meters = -width / 2.;
    vertices[0].y_meters = -width / 2.;
    vertices[1].x_meters = width / 2.;
    vertices[1].y_meters = -width / 2.;
    vertices[2].x_meters = width / 2.;
    vertices[2].y_meters = width / 2.;
    vertices[3].x_meters = -width / 2.;
    vertices[3].y_meters = width / 2.;
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
            square_feet: 100.,
            ..Default::default()
        });
        app.add_system_set(SystemSet::on_enter(AppState::WarehouseGenerator).with_system(on_enter));
        app.add_system_set(
            SystemSet::on_update(AppState::WarehouseGenerator)
                .with_system(egui_ui)
                .with_system(warehouse_generator.before(SiteMapLabel)),
        );
        app.add_system_set(SystemSet::on_exit(AppState::WarehouseGenerator).with_system(on_exit));
    }
}
