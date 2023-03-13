/*
 * Copyright (C) 2022 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

use crate::{
    interaction::{
        ChangeMode, HeadlightToggle, Hover, MoveTo, PickingBlockers, Select, SelectAnchor3D,
        SpawnPreview,
    },
    occupancy::CalculateGrid,
    recency::ChangeRank,
    site::{
        AssociatedGraphs, Change, ConsiderAssociatedGraph, ConsiderLocationTag, CurrentLevel,
        CurrentWorkspace, Delete, ExportLights, FloorVisibility, PhysicalLightToggle, SaveNavGraphs,
        SiteState, ToggleLiftDoorAvailability,
    },
    workcell::{
        LoadWorkcell
    },
    AppState,
    SaveWorkspace,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::{
    egui::{self, Button, CollapsingHeader},
    EguiContext,
};
use rmf_site_format::*;

#[cfg(not(target_arch = "wasm32"))]
use rfd::FileDialog;

pub mod create;
use create::CreateWidget;

pub mod view_layers;
use view_layers::*;

pub mod view_levels;
use view_levels::{LevelDisplay, LevelParams, ViewLevels};

pub mod view_lights;
use view_lights::*;

pub mod view_nav_graphs;
use view_nav_graphs::*;

pub mod view_occupancy;
use view_occupancy::*;

pub mod icons;
pub use icons::*;

pub mod inspector;
use inspector::{InspectorParams, InspectorWidget};

pub mod move_layer;
pub use move_layer::*;

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum UiUpdateLabel {
    DrawUi,
}

#[derive(Default)]
pub struct StandardUiLayout;

impl Plugin for StandardUiLayout {
    fn build(&self, app: &mut App) {
        app.init_resource::<Icons>()
            .init_resource::<LevelDisplay>()
            .init_resource::<NavGraphDisplay>()
            .init_resource::<LightDisplay>()
            .init_resource::<OccupancyDisplay>()
            .add_system_set(SystemSet::on_enter(SiteState::Display).with_system(init_ui_style))
            .add_system_set(
                SystemSet::on_update(SiteState::Display)
                    .with_system(site_ui_layout.label(UiUpdateLabel::DrawUi)),
            )
            .add_system_set(
                SystemSet::on_update(AppState::WorkcellEditor)
                    .with_system(workcell_ui_layout.label(UiUpdateLabel::DrawUi)),
            )
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::on_update(SiteState::Display)
                    .with_system(resolve_light_export_file)
                    .with_system(resolve_nav_graph_import_export_files),
            );
    }
}

#[derive(SystemParam)]
pub struct ChangeEvents<'w, 's> {
    pub lane_motion: EventWriter<'w, 's, Change<Motion>>,
    pub lane_reverse: EventWriter<'w, 's, Change<ReverseLane>>,
    pub name: EventWriter<'w, 's, Change<NameInSite>>,
    pub label: EventWriter<'w, 's, Change<Label>>,
    pub pose: EventWriter<'w, 's, Change<Pose>>,
    pub door: EventWriter<'w, 's, Change<DoorType>>,
    pub lift_cabin: EventWriter<'w, 's, Change<LiftCabin<Entity>>>,
    pub asset_source: EventWriter<'w, 's, Change<AssetSource>>,
    pub pixels_per_meter: EventWriter<'w, 's, Change<PixelsPerMeter>>,
    pub physical_camera_properties: EventWriter<'w, 's, Change<PhysicalCameraProperties>>,
    pub light: EventWriter<'w, 's, Change<LightKind>>,
    pub level_props: EventWriter<'w, 's, Change<LevelProperties>>,
    pub color: EventWriter<'w, 's, Change<DisplayColor>>,
    pub visibility: EventWriter<'w, 's, Change<Visibility>>,
    pub associated_graphs: EventWriter<'w, 's, Change<AssociatedGraphs<Entity>>>,
    pub location_tags: EventWriter<'w, 's, Change<LocationTags>>,
}

#[derive(SystemParam)]
pub struct FileEvents<'w, 's> {
    pub save: EventWriter<'w, 's, SaveWorkspace>,
    // TODO(luca) change into generic load workspace
    pub load_workcell: EventWriter<'w, 's, LoadWorkcell>,
}

#[derive(SystemParam)]
pub struct PanelResources<'w, 's> {
    pub level: ResMut<'w, LevelDisplay>,
    pub nav_graph: ResMut<'w, NavGraphDisplay>,
    pub light: ResMut<'w, LightDisplay>,
    pub occupancy: ResMut<'w, OccupancyDisplay>,
    _ignore: Query<'w, 's, ()>,
}

#[derive(SystemParam)]
pub struct Requests<'w, 's> {
    pub hover: ResMut<'w, Events<Hover>>,
    pub select: ResMut<'w, Events<Select>>,
    pub move_to: EventWriter<'w, 's, MoveTo>,
    pub current_level: ResMut<'w, CurrentLevel>,
    pub current_workspace: ResMut<'w, CurrentWorkspace>,
    pub change_mode: ResMut<'w, Events<ChangeMode>>,
    pub delete: EventWriter<'w, 's, Delete>,
    pub toggle_door_levels: EventWriter<'w, 's, ToggleLiftDoorAvailability>,
    pub toggle_headlights: ResMut<'w, HeadlightToggle>,
    pub toggle_physical_lights: ResMut<'w, PhysicalLightToggle>,
    pub spawn_preview: EventWriter<'w, 's, SpawnPreview>,
    pub export_lights: EventWriter<'w, 's, ExportLights>,
    pub save_nav_graphs: EventWriter<'w, 's, SaveNavGraphs>,
    pub calculate_grid: EventWriter<'w, 's, CalculateGrid>,
    pub consider_tag: EventWriter<'w, 's, ConsiderLocationTag>,
    pub consider_graph: EventWriter<'w, 's, ConsiderAssociatedGraph>,
}

#[derive(SystemParam)]
pub struct LayerEvents<'w, 's> {
    pub floors: EventWriter<'w, 's, ChangeRank<FloorMarker>>,
    pub drawings: EventWriter<'w, 's, ChangeRank<DrawingMarker>>,
    pub nav_graphs: EventWriter<'w, 's, ChangeRank<NavGraphMarker>>,
    pub change_floor_vis: EventWriter<'w, 's, Change<FloorVisibility>>,
    pub global_floor_vis: ResMut<'w, FloorVisibility>,
}

/// We collect all the events into its own SystemParam because we are not
/// allowed to receive more than one EventWriter of a given type per system call
/// (for borrow-checker reasons). Bundling them all up into an AppEvents
/// parameter at least makes the EventWriters easy to pass around.
#[derive(SystemParam)]
pub struct AppEvents<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub change: ChangeEvents<'w, 's>,
    // TODO(luca) Move to main change SystemParam once there is no more 16 param limit
    pub change_mesh_constraints: EventWriter<'w, 's, Change<MeshConstraint<Entity>>>,
    pub display: PanelResources<'w, 's>,
    pub request: Requests<'w, 's>,
    pub file_events: FileEvents<'w, 's>,
    pub layers: LayerEvents<'w, 's>,
}

fn site_ui_layout(
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    open_sites: Query<Entity, With<SiteProperties>>,
    inspector_params: InspectorParams,
    levels: LevelParams,
    lights: LightParams,
    nav_graphs: NavGraphParams,
    layers: LayersParams,
    mut events: AppEvents,
) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        CollapsingHeader::new("Levels")
                            .default_open(true)
                            .show(ui, |ui| {
                                ViewLevels::new(&levels, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Navigation Graphs")
                            .default_open(true)
                            .show(ui, |ui| {
                                ViewNavGraphs::new(&nav_graphs, &mut events).show(ui, &open_sites);
                            });
                        ui.separator();
                        // TODO(MXG): Consider combining Nav Graphs and Layers
                        CollapsingHeader::new("Layers")
                            .default_open(false)
                            .show(ui, |ui| {
                                ViewLayers::new(&layers, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Inspect")
                            .default_open(true)
                            .show(ui, |ui| {
                                InspectorWidget::new(&inspector_params, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Create")
                            .default_open(false)
                            .show(ui, |ui| {
                                CreateWidget::new(&mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Lights")
                            .default_open(false)
                            .show(ui, |ui| {
                                ViewLights::new(&lights, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Occupancy")
                            .default_open(false)
                            .show(ui, |ui| {
                                ViewOccupancy::new(&mut events).show(ui);
                            });
                    });
                });
        });

    let egui_context = egui_context.ctx_mut();
    let ui_has_focus = egui_context.wants_pointer_input()
        || egui_context.wants_keyboard_input()
        || egui_context.is_pointer_over_area();

    if let Some(picking_blocker) = &mut picking_blocker {
        picking_blocker.ui = ui_has_focus;
    }

    if ui_has_focus {
        // If the UI has focus and there were no hover events emitted by the UI,
        // then we should emit a None hover event
        if events.request.hover.is_empty() {
            events.request.hover.send(Hover(None));
        }
    }
}

fn workcell_ui_layout(
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    inspector_params: InspectorParams,
    mut events: AppEvents,
) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        CollapsingHeader::new("Inspect")
                            .default_open(true)
                            .show(ui, |ui| {
                                InspectorWidget::new(&inspector_params, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Create")
                            .default_open(true)
                            .show(ui, |ui| {
                                if ui.button("Frame").clicked() {
                                    events.request.change_mode.send(ChangeMode::To(
                                        SelectAnchor3D::create_new_point()
                                            .for_anchor(None)
                                            .into(),
                                    ));
                                }
                                // TODO(luca) Spawn window to select model asset source variant,
                                // allowing local filesystem choice for non wasm targets
                                if ui.button("Model").clicked() {
                                    let mut model = Model::default();
                                    model.source = AssetSource::Search("OpenRobotics/AdjTable".to_string());
                                    events.request.change_mode.send(ChangeMode::To(
                                        SelectAnchor3D::create_new_point()
                                            .for_model(model)
                                            .into(),
                                    ));
                                }
                            });
                        ui.separator();
                    });
                });
        });

    egui::TopBottomPanel::top("top_panel").show(egui_context.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.add(Button::new("New").shortcut_text("Ctrl+N")).clicked() {
                    println!("Loading new workcell");
                    events.file_events.load_workcell.send(LoadWorkcell {
                        workcell: Workcell::default(),
                        focus: true,
                        default_file: None,
                    });
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui.add(Button::new("Save").shortcut_text("Ctrl+S")).clicked() {
                        events.file_events.save.send(SaveWorkspace {to_file: None});
                    }
                    // TODO(luca) implement shortcuts for save as and open
                    if ui.add(Button::new("Save As").shortcut_text("Ctrl+Shift+S")).clicked() {
                        if let Some(path) = FileDialog::new().save_file() {
                            events.file_events.save.send(SaveWorkspace {to_file: Some(path)});
                        }
                    }
                    if ui.add(Button::new("Open").shortcut_text("Ctrl+O")).clicked() {
                        if let Some(path) = FileDialog::new().add_filter("Workcells", &["workcell.json"]).pick_file() {
                            let data = std::fs::read(&path);
                            if let Some(workcell) = data.ok().and_then(|d| Workcell::from_bytes(&d).ok()) {
                                events.file_events.load_workcell.send(LoadWorkcell {
                                    workcell,
                                    focus: true,
                                    default_file: Some(path),
                                });
                            }
                        }
                    }
                }
            });
        });
    });

    let egui_context = egui_context.ctx_mut();
    let ui_has_focus = egui_context.wants_pointer_input()
        || egui_context.wants_keyboard_input()
        || egui_context.is_pointer_over_area();

    if let Some(picking_blocker) = &mut picking_blocker {
        picking_blocker.ui = ui_has_focus;
    }

    if ui_has_focus {
        // If the UI has focus and there were no hover events emitted by the UI,
        // then we should emit a None hover event
        if events.request.hover.is_empty() {
            events.request.hover.send(Hover(None));
        }
    }
}

fn init_ui_style(mut egui_context: ResMut<EguiContext>) {
    // I think the default egui dark mode text color is too dim, so this changes
    // it to a brighter white.
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(250, 250, 250));
    egui_context.ctx_mut().set_visuals(visuals);
}
