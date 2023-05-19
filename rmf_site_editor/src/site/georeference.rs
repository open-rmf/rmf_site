use bevy::{prelude::*};
use bevy_egui::{
    egui::{self, Button, CollapsingHeader, Sense, panel},
    EguiContext,
};
use rmf_site_format::{Anchor, GeoReference, geo_reference};

use crate::interaction::{Selected, PickingBlockers};
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
    mut geo_events: EventReader<GeoReferenceEvent>) {

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
        });
    }
}
