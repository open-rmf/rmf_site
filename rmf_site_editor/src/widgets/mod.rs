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
        CategoryVisibility, ChangeMode, HeadlightToggle, Hover, MoveTo, PickingBlockers, Select,
        SetCategoryVisibility, SpawnPreview,
    },
    log::LogHistory,
    occupancy::CalculateGrid,
    recency::ChangeRank,
    site::{
        AlignSiteDrawings, AssociatedGraphs, BeginEditDrawing, Change, CollisionMeshMarker,
        ConsiderAssociatedGraph, ConsiderLocationTag, CurrentLevel, Delete, DrawingMarker,
        ExportLights, FinishEditDrawing, GlobalDrawingVisibility, GlobalFloorVisibility,
        LayerVisibility, MergeGroups, PhysicalLightToggle, SaveNavGraphs, SiteState, Texture,
        ToggleLiftDoorAvailability, VisualMeshMarker,
    },
    AppState, CreateNewWorkspace, CurrentWorkspace, LoadWorkspace, SaveWorkspace,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::{
    egui::{self, Button, CollapsingHeader},
    EguiContext,
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

pub mod menu_bar;
use menu_bar::*;

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

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum UiUpdateLabel {
    DrawUi,
}

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
            .init_resource::<PendingDrawing>()
            .init_resource::<PendingModel>()
            .init_resource::<SearchForFiducial>()
            .add_plugin(MenuPluginManager)
            .init_resource::<SearchForTexture>()
            .init_resource::<GroupViewModes>()
            .add_system_set(SystemSet::on_enter(AppState::MainMenu).with_system(init_ui_style))
            .add_system_set(
                SystemSet::on_update(AppState::SiteEditor)
                    .with_system(site_ui_layout.label(UiUpdateLabel::DrawUi)),
            )
            .add_system_set(
                SystemSet::on_update(AppState::WorkcellEditor)
                    .with_system(workcell_ui_layout.label(UiUpdateLabel::DrawUi)),
            )
            .add_system_set(
                SystemSet::on_update(AppState::SiteDrawingEditor)
                    .with_system(site_drawing_ui_layout.label(UiUpdateLabel::DrawUi)),
            )
            .add_system_set(
                SystemSet::on_update(AppState::SiteVisualizer)
                    .with_system(site_visualizer_ui_layout.label(UiUpdateLabel::DrawUi)),
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
    pub level_elevation: EventWriter<'w, 's, Change<LevelElevation>>,
    pub color: EventWriter<'w, 's, Change<DisplayColor>>,
    pub visibility: EventWriter<'w, 's, Change<Visibility>>,
    pub associated_graphs: EventWriter<'w, 's, Change<AssociatedGraphs<Entity>>>,
    pub location_tags: EventWriter<'w, 's, Change<LocationTags>>,
}

// We split out this new struct to deal with the 16 field limitation on
// SystemParams.
#[derive(SystemParam)]
pub struct MoreChangeEvents<'w, 's> {
    pub affiliation: EventWriter<'w, 's, Change<Affiliation<Entity>>>,
    pub search_for_fiducial: ResMut<'w, SearchForFiducial>,
    pub search_for_texture: ResMut<'w, SearchForTexture>,
    pub distance: EventWriter<'w, 's, Change<Distance>>,
    pub texture: EventWriter<'w, 's, Change<Texture>>,
    pub merge_groups: EventWriter<'w, 's, MergeGroups>,
}

#[derive(SystemParam)]
pub struct WorkcellChangeEvents<'w, 's> {
    pub mesh_constraints: EventWriter<'w, 's, Change<MeshConstraint<Entity>>>,
    pub mesh_primitives: EventWriter<'w, 's, Change<MeshPrimitive>>,
    pub name_in_workcell: EventWriter<'w, 's, Change<NameInWorkcell>>,
    pub scale: EventWriter<'w, 's, Change<Scale>>,
}

#[derive(SystemParam)]
pub struct TopMenuEvents<'w, 's> {
    pub save: EventWriter<'w, 's, SaveWorkspace>,
    pub load_workspace: EventWriter<'w, 's, LoadWorkspace>,
    pub new_workspace: EventWriter<'w, 's, CreateNewWorkspace>,
    pub diagnostic_window: ResMut<'w, DiagnosticWindowState>,
}

#[derive(SystemParam)]
pub struct PanelResources<'w, 's> {
    pub level: ResMut<'w, LevelDisplay>,
    pub nav_graph: ResMut<'w, NavGraphDisplay>,
    pub light: ResMut<'w, LightDisplay>,
    pub occupancy: ResMut<'w, OccupancyDisplay>,
    pub log_history: ResMut<'w, LogHistory>,
    pub pending_model: ResMut<'w, PendingModel>,
    pub pending_drawings: ResMut<'w, PendingDrawing>,
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
    pub layer_vis: EventWriter<'w, 's, Change<LayerVisibility>>,
    pub preferred_alpha: EventWriter<'w, 's, Change<PreferredSemiTransparency>>,
    pub global_floor_vis: EventWriter<'w, 's, Change<GlobalFloorVisibility>>,
    pub global_drawing_vis: EventWriter<'w, 's, Change<GlobalDrawingVisibility>>,
    pub begin_edit_drawing: EventWriter<'w, 's, BeginEditDrawing>,
    pub finish_edit_drawing: EventWriter<'w, 's, FinishEditDrawing>,
    pub icons: Res<'w, Icons>,
}

#[derive(SystemParam)]
pub struct VisibilityEvents<'w, 's> {
    pub doors: EventWriter<'w, 's, SetCategoryVisibility<DoorMarker>>,
    pub floors: EventWriter<'w, 's, SetCategoryVisibility<FloorMarker>>,
    pub lanes: EventWriter<'w, 's, SetCategoryVisibility<LaneMarker>>,
    pub lift_cabins: EventWriter<'w, 's, SetCategoryVisibility<LiftCabin<Entity>>>,
    pub lift_cabin_doors: EventWriter<'w, 's, SetCategoryVisibility<LiftCabinDoorMarker>>,
    pub locations: EventWriter<'w, 's, SetCategoryVisibility<LocationTags>>,
    pub fiducials: EventWriter<'w, 's, SetCategoryVisibility<FiducialMarker>>,
    pub constraints: EventWriter<'w, 's, SetCategoryVisibility<ConstraintMarker>>,
    pub measurements: EventWriter<'w, 's, SetCategoryVisibility<MeasurementMarker>>,
    pub walls: EventWriter<'w, 's, SetCategoryVisibility<WallMarker>>,
    pub visuals: EventWriter<'w, 's, SetCategoryVisibility<VisualMeshMarker>>,
    pub collisions: EventWriter<'w, 's, SetCategoryVisibility<CollisionMeshMarker>>,
}

#[derive(SystemParam)]
pub struct VisibilityResources<'w, 's> {
    pub doors: Res<'w, CategoryVisibility<DoorMarker>>,
    pub floors: Res<'w, CategoryVisibility<FloorMarker>>,
    pub lanes: Res<'w, CategoryVisibility<LaneMarker>>,
    pub lift_cabins: Res<'w, CategoryVisibility<LiftCabin<Entity>>>,
    pub lift_cabin_doors: Res<'w, CategoryVisibility<LiftCabinDoorMarker>>,
    pub locations: Res<'w, CategoryVisibility<LocationTags>>,
    pub fiducials: Res<'w, CategoryVisibility<FiducialMarker>>,
    pub constraints: Res<'w, CategoryVisibility<ConstraintMarker>>,
    pub measurements: Res<'w, CategoryVisibility<MeasurementMarker>>,
    pub walls: Res<'w, CategoryVisibility<WallMarker>>,
    pub visuals: Res<'w, CategoryVisibility<VisualMeshMarker>>,
    pub collisions: Res<'w, CategoryVisibility<CollisionMeshMarker>>,
    _ignore: Query<'w, 's, ()>,
}

#[derive(SystemParam)]
pub struct VisibilityParameters<'w, 's> {
    events: VisibilityEvents<'w, 's>,
    resources: VisibilityResources<'w, 's>,
}

#[derive(SystemParam)]
pub struct MenuParams<'w, 's> {
    menus: Query<'w, 's, (&'static Menu, Entity)>,
    menu_items: Query<'w, 's, (&'static mut MenuItem, Option<&'static MenuDisabled>)>,
    extension_events: EventWriter<'w, 's, MenuEvent>,
    view_menu: Res<'w, ViewMenu>,
}

/// We collect all the events into its own SystemParam because we are not
/// allowed to receive more than one EventWriter of a given type per system call
/// (for borrow-checker reasons). Bundling them all up into an AppEvents
/// parameter at least makes the EventWriters easy to pass around.
#[derive(SystemParam)]
pub struct AppEvents<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub change: ChangeEvents<'w, 's>,
    pub change_more: MoreChangeEvents<'w, 's>,
    pub workcell_change: WorkcellChangeEvents<'w, 's>,
    pub display: PanelResources<'w, 's>,
    pub request: Requests<'w, 's>,
    pub top_menu_events: TopMenuEvents<'w, 's>,
    pub layers: LayerEvents<'w, 's>,
    pub new_model: NewModelParams<'w, 's>,
    pub app_state: ResMut<'w, State<AppState>>,
    pub visibility_parameters: VisibilityParameters<'w, 's>,
    pub align_site: EventWriter<'w, 's, AlignSiteDrawings>,
}

fn site_ui_layout(
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    open_sites: Query<Entity, With<NameOfSite>>,
    inspector_params: InspectorParams,
    create_params: CreateParams,
    levels: LevelParams,
    lights: LightParams,
    nav_graphs: NavGraphParams,
    mut diagnostic_params: DiagnosticParams,
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
                            if let Err(err) =
                                events.app_state.overwrite_set(AppState::SiteVisualizer)
                            {
                                error!("Failed to switch to full site visualization: {err}");
                            }
                        }
                    });
                });
        });

    top_menu_bar(
        &mut egui_context,
        &mut events.file_events,
        &mut events.visibility_parameters,
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

    if events.new_model.asset_gallery_status.show {
        egui::SidePanel::left("left_panel")
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
    mut egui_context: ResMut<EguiContext>,
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
        &mut egui_context,
        &mut events.file_events,
        &mut events.visibility_parameters,
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
    mut egui_context: ResMut<EguiContext>,
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
                                events.align_site.send(AlignSiteDrawings(site));
                            }
                        }
                        if ui.add(Button::image_and_text(
                            events.layers.icons.exit.egui(),
                            [18., 18.],
                            "Return to site editor"
                        )).clicked() {
                            if let Err(err) = events.app_state.overwrite_set(AppState::SiteEditor) {
                                error!("Failed to return to site editor: {err}");
                            }
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
        &mut egui_context,
        &mut events.file_events,
        &mut events.visibility_parameters,
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
    mut egui_context: ResMut<EguiContext>,
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
        &mut egui_context,
        &mut events.file_events,
        &mut events.visibility_parameters,
        &file_menu,
        &top_level_components,
        &children,
        &mut menu_params,
    );

    if events.new_model.asset_gallery_status.show {
        egui::SidePanel::left("left_panel")
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

fn init_ui_style(mut egui_context: ResMut<EguiContext>) {
    // I think the default egui dark mode text color is too dim, so this changes
    // it to a brighter white.
    let mut visuals = egui::Visuals::dark();
    visuals.override_text_color = Some(egui::Color32::from_rgb(250, 250, 250));
    egui_context.ctx_mut().set_visuals(visuals);
}
