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
        ChangeMode, HeadlightToggle, Hover, MoveTo, PickingBlockers, Select, SpawnPreview,
    },
    log::LogHistory,
    occupancy::CalculateGrid,
    recency::ChangeRank,
    site::{
        AlignSiteDrawings, AssociatedGraphs, BeginEditDrawing, Change, CollisionMeshMarker,
        ConsiderAssociatedGraph, ConsiderLocationTag, CurrentLevel, Delete, DrawingMarker,
        ExportLights, FinishEditDrawing, GlobalDrawingVisibility, GlobalFloorVisibility,
        JointProperties, LayerVisibility, MergeGroups, PhysicalLightToggle, SaveNavGraphs, Texture,
        ToggleLiftDoorAvailability, VisualMeshMarker,
    },
    workcell::CreateJoint,
    AppState, CreateNewWorkspace, CurrentWorkspace, LoadWorkspace, SaveWorkspace,
    ValidateWorkspace,
};
use bevy::{ecs::query::Has, ecs::system::SystemParam, prelude::*};
use bevy_egui::{
    egui::{self, Button, CollapsingHeader},
    EguiContexts,
};
use rmf_site_format::*;

pub mod create;
use create::*;

pub mod menu_bar;
use menu_bar::*;

pub mod view_groups;
use view_groups::*;

pub mod diagnostic_window;
use diagnostic_window::*;

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

pub mod console;
pub use console::*;

pub mod icons;
pub use icons::*;

pub mod inspector;
use inspector::{InspectorParams, InspectorWidget, SearchForFiducial, SearchForTexture};

pub mod move_layer;
pub use move_layer::*;

pub mod new_model;
pub use new_model::*;

#[derive(Resource, Clone, Default)]
pub struct PendingDrawing {
    pub source: AssetSource,
    pub recall_source: RecallAssetSource,
}

#[derive(Resource, Clone, Default)]
pub struct PendingModel {
    pub source: AssetSource,
    pub recall_source: RecallAssetSource,
    pub scale: Scale,
}

#[derive(Default)]
pub struct StandardUiLayout;

impl Plugin for StandardUiLayout {
    fn build(&self, app: &mut App) {
        app.init_resource::<Icons>()
            .init_resource::<LevelDisplay>()
            .init_resource::<NavGraphDisplay>()
            .init_resource::<LightDisplay>()
            .init_resource::<AssetGalleryStatus>()
            .init_resource::<OccupancyDisplay>()
            .init_resource::<DiagnosticWindowState>()
            .init_resource::<PendingDrawing>()
            .init_resource::<PendingModel>()
            .init_resource::<SearchForFiducial>()
            .add_plugins(MenuPluginManager)
            .init_resource::<SearchForTexture>()
            .init_resource::<GroupViewModes>()
            .add_systems(Startup, init_ui_style)
            .add_systems(
                Update,
                site_ui_layout.run_if(in_state(AppState::SiteEditor)),
            )
            .add_systems(
                Update,
                workcell_ui_layout.run_if(in_state(AppState::WorkcellEditor)),
            )
            .add_systems(
                Update,
                site_drawing_ui_layout.run_if(in_state(AppState::SiteDrawingEditor)),
            )
            .add_systems(
                Update,
                site_visualizer_ui_layout.run_if(in_state(AppState::SiteVisualizer)),
            )
            .add_systems(
                PostUpdate,
                (
                    resolve_light_export_file,
                    resolve_nav_graph_import_export_files,
                )
                    .run_if(AppState::in_site_mode()),
            );
    }
}

#[derive(SystemParam)]
pub struct ChangeEvents<'w> {
    pub lane_motion: EventWriter<'w, Change<Motion>>,
    pub lane_reverse: EventWriter<'w, Change<ReverseLane>>,
    pub name: EventWriter<'w, Change<NameInSite>>,
    pub pose: EventWriter<'w, Change<Pose>>,
    pub door: EventWriter<'w, Change<DoorType>>,
    pub lift_cabin: EventWriter<'w, Change<LiftCabin<Entity>>>,
    pub asset_source: EventWriter<'w, Change<AssetSource>>,
    pub pixels_per_meter: EventWriter<'w, Change<PixelsPerMeter>>,
    pub physical_camera_properties: EventWriter<'w, Change<PhysicalCameraProperties>>,
    pub light: EventWriter<'w, Change<LightKind>>,
    pub level_elevation: EventWriter<'w, Change<LevelElevation>>,
    pub color: EventWriter<'w, Change<DisplayColor>>,
    pub visibility: EventWriter<'w, Change<Visibility>>,
    pub associated_graphs: EventWriter<'w, Change<AssociatedGraphs<Entity>>>,
    pub location_tags: EventWriter<'w, Change<LocationTags>>,
    pub affiliation: EventWriter<'w, Change<Affiliation<Entity>>>,
    pub search_for_fiducial: ResMut<'w, SearchForFiducial>,
    pub search_for_texture: ResMut<'w, SearchForTexture>,
    pub distance: EventWriter<'w, Change<Distance>>,
    pub texture: EventWriter<'w, Change<Texture>>,
    pub joint_properties: EventWriter<'w, Change<JointProperties>>,
    pub merge_groups: EventWriter<'w, MergeGroups>,
    pub filtered_issues: EventWriter<'w, Change<FilteredIssues<Entity>>>,
    pub filtered_issue_kinds: EventWriter<'w, Change<FilteredIssueKinds>>,
}

#[derive(SystemParam)]
pub struct WorkcellChangeEvents<'w> {
    pub mesh_constraints: EventWriter<'w, Change<MeshConstraint<Entity>>>,
    pub name_in_workcell: EventWriter<'w, Change<NameInWorkcell>>,
    pub workcell_name: EventWriter<'w, Change<NameOfWorkcell>>,
    pub scale: EventWriter<'w, Change<Scale>>,
    pub primitive_shapes: EventWriter<'w, Change<PrimitiveShape>>,
}

#[derive(SystemParam)]
pub struct FileEvents<'w> {
    pub save: EventWriter<'w, SaveWorkspace>,
    pub load_workspace: EventWriter<'w, LoadWorkspace>,
    pub new_workspace: EventWriter<'w, CreateNewWorkspace>,
    pub diagnostic_window: ResMut<'w, DiagnosticWindowState>,
}

#[derive(SystemParam)]
pub struct PanelResources<'w> {
    pub level: ResMut<'w, LevelDisplay>,
    pub nav_graph: ResMut<'w, NavGraphDisplay>,
    pub light: ResMut<'w, LightDisplay>,
    pub occupancy: ResMut<'w, OccupancyDisplay>,
    pub log_history: ResMut<'w, LogHistory>,
    pub pending_model: ResMut<'w, PendingModel>,
    pub pending_drawings: ResMut<'w, PendingDrawing>,
}

#[derive(SystemParam)]
pub struct Requests<'w> {
    pub hover: ResMut<'w, Events<Hover>>,
    pub select: ResMut<'w, Events<Select>>,
    pub move_to: EventWriter<'w, MoveTo>,
    pub current_level: ResMut<'w, CurrentLevel>,
    pub current_workspace: ResMut<'w, CurrentWorkspace>,
    pub change_mode: ResMut<'w, Events<ChangeMode>>,
    pub delete: EventWriter<'w, Delete>,
    pub toggle_door_levels: EventWriter<'w, ToggleLiftDoorAvailability>,
    pub toggle_headlights: ResMut<'w, HeadlightToggle>,
    pub toggle_physical_lights: ResMut<'w, PhysicalLightToggle>,
    pub spawn_preview: EventWriter<'w, SpawnPreview>,
    pub export_lights: EventWriter<'w, ExportLights>,
    pub save_nav_graphs: EventWriter<'w, SaveNavGraphs>,
    pub calculate_grid: EventWriter<'w, CalculateGrid>,
    pub consider_tag: EventWriter<'w, ConsiderLocationTag>,
    pub consider_graph: EventWriter<'w, ConsiderAssociatedGraph>,
    pub align_site: EventWriter<'w, AlignSiteDrawings>,
    pub validate_workspace: EventWriter<'w, ValidateWorkspace>,
    pub create_joint: EventWriter<'w, CreateJoint>,
}

#[derive(SystemParam)]
pub struct LayerEvents<'w> {
    pub floors: EventWriter<'w, ChangeRank<FloorMarker>>,
    pub drawings: EventWriter<'w, ChangeRank<DrawingMarker>>,
    pub nav_graphs: EventWriter<'w, ChangeRank<NavGraphMarker>>,
    pub layer_vis: EventWriter<'w, Change<LayerVisibility>>,
    pub preferred_alpha: EventWriter<'w, Change<PreferredSemiTransparency>>,
    pub global_floor_vis: EventWriter<'w, Change<GlobalFloorVisibility>>,
    pub global_drawing_vis: EventWriter<'w, Change<GlobalDrawingVisibility>>,
    pub begin_edit_drawing: EventWriter<'w, BeginEditDrawing>,
    pub finish_edit_drawing: EventWriter<'w, FinishEditDrawing>,
    pub icons: Res<'w, Icons>,
}

#[derive(SystemParam)]
pub struct MenuParams<'w, 's> {
    state: Res<'w, State<AppState>>,
    menus: Query<'w, 's, (&'static Menu, Entity)>,
    menu_items: Query<'w, 's, (&'static mut MenuItem, Has<MenuDisabled>)>,
    menu_states: Query<'w, 's, Option<&'static MenuVisualizationStates>>,
    extension_events: EventWriter<'w, MenuEvent>,
    view_menu: Res<'w, ViewMenu>,
}

/// We collect all the events into its own SystemParam because we are not
/// allowed to receive more than one EventWriter of a given type per system call
/// (for borrow-checker reasons). Bundling them all up into an AppEvents
/// parameter at least makes the EventWriters easy to pass around.
#[derive(SystemParam)]
pub struct AppEvents<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub change: ChangeEvents<'w>,
    pub workcell_change: WorkcellChangeEvents<'w>,
    pub display: PanelResources<'w>,
    pub request: Requests<'w>,
    pub file_events: FileEvents<'w>,
    pub layers: LayerEvents<'w>,
    pub new_model: NewModelParams<'w>,
    pub app_state: Res<'w, State<AppState>>,
    pub next_app_state: ResMut<'w, NextState<AppState>>,
}

fn site_ui_layout(
    mut egui_context: EguiContexts,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    open_sites: Query<Entity, With<NameOfSite>>,
    inspector_params: InspectorParams,
    create_params: CreateParams,
    levels: LevelParams,
    lights: LightParams,
    nav_graphs: NavGraphParams,
    diagnostic_params: DiagnosticParams,
    layers: LayersParams,
    mut groups: GroupParams,
    mut events: AppEvents,
    file_menu: Res<FileMenu>,
    children: Query<&Children>,
    top_level_components: Query<(), Without<Parent>>,
    mut menu_params: MenuParams,
) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(300.0)
        .show(egui_context.ctx_mut(), |ui| {
            egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        CollapsingHeader::new("Levels")
                            .default_open(true)
                            .show(ui, |ui| {
                                ViewLevels::new(&levels, &mut events)
                                    .for_editing_visibility()
                                    .show(ui);
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
                                CreateWidget::new(&create_params, &mut events).show(ui);
                            });
                        ui.separator();
                        CollapsingHeader::new("Groups")
                            .default_open(false)
                            .show(ui, |ui| {
                                ViewGroups::new(&mut groups, &mut events).show(ui);
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
                        if ui.add(Button::new("Building preview")).clicked() {
                            events.next_app_state.set(AppState::SiteVisualizer);
                        }
                    });
                });
        });

    top_menu_bar(
        egui_context.ctx_mut(),
        &mut events.file_events,
        &file_menu,
        &top_level_components,
        &children,
        &mut menu_params,
    );

    egui::TopBottomPanel::bottom("log_console")
        .resizable(true)
        .min_height(30.)
        .max_height(300.)
        .show(egui_context.ctx_mut(), |ui| {
            ui.add_space(10.0);
            ConsoleWidget::new(&mut events).show(ui);
        });

    if events.file_events.diagnostic_window.show {
        egui::SidePanel::left("diagnostic_window")
            .resizable(true)
            .exact_width(320.0)
            .show(egui_context.ctx_mut(), |ui| {
                DiagnosticWindow::new(&mut events, &diagnostic_params).show(ui);
            });
    }
    if events.new_model.asset_gallery_status.show {
        egui::SidePanel::left("asset_gallery")
            .resizable(true)
            .exact_width(320.0)
            .show(egui_context.ctx_mut(), |ui| {
                NewModel::new(&mut events).show(ui);
            });
    }

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

fn site_drawing_ui_layout(
    mut egui_context: EguiContexts,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    inspector_params: InspectorParams,
    create_params: CreateParams,
    mut events: AppEvents,
    file_menu: Res<FileMenu>,
    children: Query<&Children>,
    top_level_components: Query<(), Without<Parent>>,
    mut menu_params: MenuParams,
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
                                CreateWidget::new(&create_params, &mut events).show(ui);
                            });
                        ui.separator();
                        if ui
                            .add(Button::image_and_text(
                                events.layers.icons.exit.egui(),
                                [18., 18.],
                                "Return to site editor",
                            ))
                            .clicked()
                        {
                            events
                                .layers
                                .finish_edit_drawing
                                .send(FinishEditDrawing(None));
                        }
                    });
                });
        });

    egui::TopBottomPanel::bottom("log_console")
        .resizable(true)
        .min_height(30.)
        .max_height(300.)
        .show(egui_context.ctx_mut(), |ui| {
            ui.add_space(10.0);
            ConsoleWidget::new(&mut events).show(ui);
        });

    top_menu_bar(
        egui_context.ctx_mut(),
        &mut events.file_events,
        &file_menu,
        &top_level_components,
        &children,
        &mut menu_params,
    );

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

fn site_visualizer_ui_layout(
    mut egui_context: EguiContexts,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    mut events: AppEvents,
    levels: LevelParams,
    file_menu: Res<FileMenu>,
    top_level_components: Query<(), Without<Parent>>,
    children: Query<&Children>,
    mut menu_params: MenuParams,
) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .default_width(300.0)
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
                        if ui.add(Button::image_and_text(
                            events.layers.icons.alignment.egui(),
                            [18., 18.],
                            "Align Drawings",
                        ))
                            .on_hover_text("Align all drawings in the site based on their fiducials and measurements")
                            .clicked()
                        {
                            if let Some(site) = events.request.current_workspace.root {
                                events.request.align_site.send(AlignSiteDrawings(site));
                            }
                        }
                        if ui.add(Button::image_and_text(
                            events.layers.icons.exit.egui(),
                            [18., 18.],
                            "Return to site editor"
                        )).clicked() {
                            events.next_app_state.set(AppState::SiteEditor);
                        }
                    });
                });
        });

    egui::TopBottomPanel::bottom("log_console")
        .resizable(true)
        .min_height(30.)
        .max_height(300.)
        .show(egui_context.ctx_mut(), |ui| {
            ui.add_space(10.0);
            ConsoleWidget::new(&mut events).show(ui);
        });

    top_menu_bar(
        egui_context.ctx_mut(),
        &mut events.file_events,
        &file_menu,
        &top_level_components,
        &children,
        &mut menu_params,
    );

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
    mut egui_context: EguiContexts,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    inspector_params: InspectorParams,
    create_params: CreateParams,
    mut events: AppEvents,
    file_menu: Res<FileMenu>,
    top_level_components: Query<(), Without<Parent>>,
    children: Query<&Children>,
    mut menu_params: MenuParams,
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
                                CreateWidget::new(&create_params, &mut events).show(ui);
                            });
                        ui.separator();
                    });
                });
        });

    egui::TopBottomPanel::bottom("log_console")
        .resizable(true)
        .min_height(30.)
        .max_height(300.)
        .show(egui_context.ctx_mut(), |ui| {
            ui.add_space(10.0);
            ConsoleWidget::new(&mut events).show(ui);
        });

    top_menu_bar(
        egui_context.ctx_mut(),
        &mut events.file_events,
        &file_menu,
        &top_level_components,
        &children,
        &mut menu_params,
    );

    if events.new_model.asset_gallery_status.show {
        egui::SidePanel::left("asset_gallery")
            .resizable(true)
            .exact_width(320.0)
            .show(egui_context.ctx_mut(), |ui| {
                NewModel::new(&mut events).show(ui);
            });
    }

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

fn init_ui_style(mut egui_context: EguiContexts) {
    // I think the default egui dark mode text color is too dim, so this changes
    // it to a brighter white.
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(250, 250, 250));
    egui_context.ctx_mut().set_visuals(visuals);
}
