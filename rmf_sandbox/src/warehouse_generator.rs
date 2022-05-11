use crate::level::Level;
use crate::site_map::{SiteMap, SiteMapLabel};
use crate::vertex::Vertex;
use crate::wall::Wall;
use crate::AppState;

use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};

#[derive(Clone, Default, PartialEq)]
pub struct WarehouseParams {
    pub square_feet: f64,
}

#[derive(Default)]
pub struct WarehouseState {
    pub requested: WarehouseParams,
    pub spawned: WarehouseParams,
}

fn egui_ui(mut egui_context: ResMut<EguiContext>, mut warehouse_state: ResMut<WarehouseState>) {
    egui::SidePanel::left("left").show(egui_context.ctx_mut(), |ui| {
        ui.heading("Warehouse Generator");
        ui.add_space(10.);
        ui.add(
            egui::Slider::new(&mut warehouse_state.requested.square_feet, 100.0..=1000.0)
                .text("Square feet"),
        );
    });
}

fn warehouse_generator(
    mut commands: Commands,
    mut sm: ResMut<SiteMap>,
    mut warehouse_state: ResMut<WarehouseState>,
    mesh_query: Query<(Entity, &Handle<Mesh>)>,
) {
    if warehouse_state.requested != warehouse_state.spawned {
        // first, despawn all existing mesh entities
        for entity_mesh in mesh_query.iter() {
            let (entity, _mesh) = entity_mesh;
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
        level.walls.push(Wall { start: 0, end: 1 });
        level.walls.push(Wall { start: 1, end: 2 });
        level.walls.push(Wall { start: 2, end: 3 });
        level.walls.push(Wall { start: 3, end: 0 });
        sm.levels.clear();
        sm.levels.push(level);

        warehouse_state.spawned = warehouse_state.requested.clone();
    }
}

fn on_enter(mut commands: Commands) {
    let mut site_map = SiteMap::default();
    site_map.site_name = "new site".to_string();
    commands.insert_resource(site_map);
}

fn on_exit(mut commands: Commands) {
    commands.remove_resource::<SiteMap>();
}

pub struct WarehouseGeneratorPlugin;

impl Plugin for WarehouseGeneratorPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(WarehouseState {
            requested: WarehouseParams { square_feet: 100. },
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
