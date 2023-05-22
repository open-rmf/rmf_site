use bevy::{prelude::*};
use bevy_egui::{
    egui::{self, Button, CollapsingHeader, Sense, panel},
    EguiContext,
};
use rmf_site_format::{Anchor, GeoReference, geo_reference, AssetSource};
use std::f32::consts::PI;

use crate::{interaction::{Selected, PickingBlockers}, OSMTile};
pub struct GeoReferenceEvent{}

enum SelectionMode {
    AnchorSelected(Entity),
    AnchorSelect,
    NoSelection
}

impl Default for SelectionMode {
    fn default() -> Self {
        SelectionMode::NoSelection
    }
}

fn selection_mode_labels(mode: &SelectionMode) -> String {
    match mode {
        SelectionMode::AnchorSelected(entity) => {
            format!("Anchor {:?}", entity)
        },
        SelectionMode::AnchorSelect => {
            "Click the anchor you want to use".to_owned()
        },
        SelectionMode::NoSelection => {
            "Select Anchor".to_owned()
        }
    }
}

#[derive(Default, Resource)]
pub struct GeoReferencePanelState {
    enabled: bool,
    latitude_raw_input1: f32,
    latitude_raw_input2: f32,
    longitude_raw_input1: f32,
    longitude_raw_input2: f32,
    selection_mode1: SelectionMode,
    selection_mode2: SelectionMode
}

pub fn add_georeference(
    mut georef_anchors: Query<(&Anchor, &GeoReference<Entity>, Entity)>,
    selected_anchors: Query<(&Anchor, &Selected, Entity)>,
    mut panel_state: Local<GeoReferencePanelState>,
    mut egui_context: ResMut<EguiContext>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    asset_server: Res<AssetServer>,
    mut geo_events: EventReader<GeoReferenceEvent>,
    mut commands: Commands) {

    for _event in geo_events.iter() {
        panel_state.enabled = true;
    }

    let selected: Vec<_> = selected_anchors.iter().filter(|(_anchor, selected, _entity)| {
        selected.is_selected
    }).collect();

    if panel_state.enabled
    {
        // Draw UI
        egui::Window::new("Geographic Reference").show(egui_context.ctx_mut(), |ui| {
            ui.horizontal(|ui| {
                ui.label("Reference Anchor 1: ");
                if ui.button(selection_mode_labels(&panel_state.selection_mode1)).clicked() {
                    if selected.len() == 0 {
                        panel_state.selection_mode1 = SelectionMode::AnchorSelect;
                    }
                    else {
                        panel_state.selection_mode1 = SelectionMode::AnchorSelected(selected[0].2);
                    }
                }
                ui.label("Latitude: ");
                ui.add(egui::DragValue::new(&mut panel_state.latitude_raw_input1).speed(0.00001));
                ui.label("Longitude: ");
                ui.add(egui::DragValue::new(&mut panel_state.longitude_raw_input1).speed(0.00001));
            });
            ui.horizontal(|ui| {
                ui.label("Reference Anchor 2: ");
                if ui.button(selection_mode_labels(&panel_state.selection_mode2)).clicked() {
                    if selected.len() == 0 {
                        panel_state.selection_mode2 = SelectionMode::AnchorSelect;
                    }
                    else {
                        panel_state.selection_mode2 = SelectionMode::AnchorSelected(selected[0].2);
                    }
                }
                ui.label("Latitude: ");
                ui.add(egui::DragValue::new(&mut panel_state.latitude_raw_input2).speed(0.001));
                ui.label("Longitude: ");
                ui.add(egui::DragValue::new(&mut panel_state.longitude_raw_input2).speed(0.001));
            });

            if selected.len() != 0 && matches!(panel_state.selection_mode2, SelectionMode::AnchorSelect) {
                panel_state.selection_mode2 = SelectionMode::AnchorSelected(selected[0].2);
            }

            if selected.len() != 0 && matches!(panel_state.selection_mode1, SelectionMode::AnchorSelect) {
                panel_state.selection_mode1 = SelectionMode::AnchorSelected(selected[0].2);
            }
            if ui.button("Preview Map").clicked() {
                
                println!("Preview");

                let ulsan = (35.53330554519475, 129.38965867799482);
                let tile = OSMTile::from_latlon(18, ulsan.0, ulsan.1);
                let tile_size = tile.tile_size();
                println!("{:?}", tile_size);
                /*let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
                    tile_size.0,
                    tile_size.1,
                ))));
                
                let texture_handle: Handle<Image> = asset_server.load(String::from(
                    &AssetSource::OSMSlippyMap(ulsan.0, ulsan.1)));
                let material_handle = materials.add(StandardMaterial {
                    base_color_texture: Some(texture_handle.clone()),
                    alpha_mode: AlphaMode::Blend,
                    unlit: true,
                    ..default()
                });
                
                commands.spawn(PbrBundle {
                    mesh: quad_handle,
                    material: material_handle,
                    ..default()
                });*/
                let quad_handle = meshes.add(Mesh::from(shape::Quad::new(Vec2::new(
                    tile_size.0,
                    tile_size.1,
                ))));
                commands.spawn(PbrBundle {
                    mesh: quad_handle,
                    material: materials.add(Color::rgb(0.3, 0.5, 0.3).into()),
                    ..default()
                });
            }
        });
    }
}
