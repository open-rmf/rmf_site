use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};
use super::level::Level;
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
            egui::Slider::new(&mut warehouse_state.requested.square_feet, 100.0..=1000.0)
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
        warehouse_state.spawned = warehouse_state.requested.clone();
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
