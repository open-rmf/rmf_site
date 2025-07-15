/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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
    interaction::{AnchorSelection, ObjectPlacement},
    site::{
        Affiliation, AssetSource, Category, DefaultFile, DrawingBundle, DrawingProperties, Group,
        IsStatic, Members, ModelDescriptionBundle, ModelInstance, ModelMarker, ModelProperty,
        NameInSite, Recall, RecallAssetSource, Scale,
    },
    widgets::{AssetGalleryStatus, Icons, InspectAssetSourceComponent, InspectScaleComponent},
    AppState, CurrentWorkspace,
};

use bevy::ecs::{
    hierarchy::ChildOf,
    system::{SystemParam, SystemState},
};
use bevy::prelude::*;
use bevy_egui::egui::{self, Button, ComboBox, Ui};
use rmf_site_egui::*;

/// This plugin creates a standard set of site object creation buttons
#[derive(Default)]
pub struct StandardCreationPlugin {}

impl Plugin for StandardCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            LaneCreationPlugin::default(),
            LocationCreationPlugin::default(),
            WallCreationPlugin::default(),
            DoorCreationPlugin::default(),
            LiftCreationPlugin::default(),
            FloorCreationPlugin::default(),
            FiducialCreationPlugin::default(),
            MeasurementCreationPlugin::default(),
            DrawingCreationPlugin::default(),
            ModelCreationPlugin::default(),
            BrowseFuelTogglePlugin::default(),
        ));
    }
}

/// Add a widget for lane creation
#[derive(Default)]
pub struct LaneCreationPlugin {}

impl Plugin for LaneCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<LaneCreation>::new());
    }
}

#[derive(SystemParam)]
struct LaneCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for LaneCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "↔", "Lane") {
                params.anchor_selection.create_lanes();
            }
        }
    }
}

/// Add widget for location creation
#[derive(Default)]
pub struct LocationCreationPlugin {}

impl Plugin for LocationCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<LocationCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct LocationCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for LocationCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "📌", "Location") {
                params.anchor_selection.create_location();
            }
        }
    }
}

/// Add widget for wall creation
#[derive(Default)]
pub struct WallCreationPlugin {}

impl Plugin for WallCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<WallCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct WallCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for WallCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "■", "Wall") {
                params.anchor_selection.create_walls();
            }
        }
    }
}

/// Add widget for door creation
#[derive(Default)]
pub struct DoorCreationPlugin {}

impl Plugin for DoorCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<DoorCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct DoorCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for DoorCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "🚪", "Door") {
                params.anchor_selection.create_door();
            }
        }
    }
}

/// Add widget for lift creation
#[derive(Default)]
pub struct LiftCreationPlugin {}

impl Plugin for LiftCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<LiftCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct LiftCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for LiftCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "⬍", "Lift") {
                params.anchor_selection.create_lift();
            }
        }
    }
}

/// Add widget for floor creation
#[derive(Default)]
pub struct FloorCreationPlugin {}

impl Plugin for FloorCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<FloorCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct FloorCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for FloorCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteEditor = params.app_state.get() {
            if button_clicked(ui, "✏", "Floor") {
                params.anchor_selection.create_floor();
            }
        }
    }
}

/// Add widget for fiducial creation
#[derive(Default)]
pub struct FiducialCreationPlugin {}

impl Plugin for FiducialCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<FiducialCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct FiducialCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for FiducialCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        match params.app_state.get() {
            AppState::SiteEditor => {
                if button_clicked(ui, "☉", "Fiducial") {
                    params.anchor_selection.create_site_fiducial();
                }
            }
            AppState::SiteDrawingEditor => {
                if button_clicked(ui, "☉", "Fiducial") {
                    params.anchor_selection.create_drawing_fiducial();
                }
            }
            _ => {
                return;
            }
        }
    }
}

/// Add widget for measurement creation
#[derive(Default)]
pub struct MeasurementCreationPlugin {}

impl Plugin for MeasurementCreationPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<MeasurementCreation>::new());
    }
}

#[derive(SystemParam)]
pub struct MeasurementCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    anchor_selection: AnchorSelection<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for MeasurementCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if let AppState::SiteDrawingEditor = params.app_state.get() {
            if button_clicked(ui, "📏", "Measurement") {
                params.anchor_selection.create_measurements();
            }
        }
    }
}

#[derive(Default)]
pub struct DrawingCreationPlugin {}

impl Plugin for DrawingCreationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingDrawing>()
            .add_plugins(HeaderTilePlugin::<DrawingCreation>::new());
    }
}

#[derive(Clone, Default, Resource)]
struct PendingDrawing {
    pub source: AssetSource,
    pub recall_source: RecallAssetSource,
}

#[derive(SystemParam)]
pub struct DrawingCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    pending: ResMut<'w, PendingDrawing>,
    current_workspace: Res<'w, CurrentWorkspace>,
    default_file: Query<'w, 's, &'static DefaultFile>,
    commands: Commands<'w, 's>,
    icons: Res<'w, Icons>,
}

impl<'w, 's> WidgetSystem<Tile> for DrawingCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        match params.app_state.get() {
            AppState::SiteEditor => {
                ui.menu_button("🖼️", |ui| {
                    // The menu_button style isn't good for general widgets, so
                    // we reset the style before drawing the inner widgets.
                    ui.reset_style();

                    ui.vertical(|ui| {
                        egui::Resize::default()
                            .default_width(300.0)
                            .default_height(0.0)
                            .show(ui, |ui| {
                                let default_file = params
                                    .current_workspace
                                    .root
                                    .map(|e| params.default_file.get(e).ok())
                                    .flatten();
                                ui.add_space(10.0);
                                if let Some(new_asset_source) = InspectAssetSourceComponent::new(
                                    &params.pending.source,
                                    &params.pending.recall_source,
                                    default_file,
                                )
                                .show(ui)
                                {
                                    params.pending.recall_source.remember(&new_asset_source);
                                    params.pending.source = new_asset_source;
                                }
                                ui.add_space(5.0);
                                if ui
                                    .add(
                                        Button::image_and_text(
                                            params.icons.add.egui(),
                                            "Add Drawing",
                                        ), // .min_size(egui::Vec2::new(0.0, 0.0))
                                    )
                                    .clicked()
                                {
                                    params.commands.spawn(DrawingBundle::new(DrawingProperties {
                                        source: params.pending.source.clone(),
                                        ..default()
                                    }));
                                    ui.close_menu();
                                }
                                ui.add_space(10.0);
                            });
                    });
                })
                .response
                .on_hover_text("Drawing");
            }
            _ => {
                return;
            }
        }
    }
}

#[derive(Default)]
pub struct ModelCreationPlugin {}

impl Plugin for ModelCreationPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingModelDescription>()
            .add_plugins(HeaderTilePlugin::<ModelCreation>::new());
    }
}

#[derive(Clone, Resource)]
pub struct PendingModelDescription {
    pub selected: Option<Entity>,
    pub name: String,
    pub source: AssetSource,
    pub recall_source: RecallAssetSource,
    pub scale: Scale,
}

impl Default for PendingModelDescription {
    fn default() -> Self {
        Self {
            selected: None,
            name: "<Unnamed Description>".to_string(),
            source: AssetSource::default(),
            recall_source: RecallAssetSource::default(),
            scale: Scale::default(),
        }
    }
}

#[derive(SystemParam)]
pub struct ModelCreation<'w, 's> {
    app_state: Res<'w, State<AppState>>,
    pending: ResMut<'w, PendingModelDescription>,
    current_workspace: Res<'w, CurrentWorkspace>,
    default_file: Query<'w, 's, &'static DefaultFile>,
    icons: Res<'w, Icons>,
    children: Query<'w, 's, &'static Children>,
    descriptions: Query<'w, 's, &'static NameInSite, (With<ModelMarker>, With<Group>)>,
    object_placement: ObjectPlacement<'w, 's>,
    next_instance_name: GetNextInstanceName<'w, 's>,
    commands: Commands<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for ModelCreation<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        let Some(site_entity) = params.current_workspace.root else {
            return;
        };

        match params.app_state.get() {
            AppState::SiteEditor => {
                ui.menu_button("🔩", |ui| {
                    // The menu_button style isn't good for general widgets, so
                    // we reset the style before drawing the inner widgets.
                    ui.reset_style();

                    let mut add_instance = None;
                    ui.vertical(|ui| {
                        egui::Resize::default()
                            .default_width(300.0)
                            .default_height(0.0)
                            .show(ui, |ui| {
                                ui.add_space(10.0);
                                // Section for spawning a new instance from an existing description
                                ui.horizontal(|ui| {
                                    if ui
                                        .add_enabled(
                                            params.pending.selected.is_some(),
                                            egui::ImageButton::new(params.icons.add.egui()),
                                        )
                                        .on_hover_text("Add Instance")
                                        .clicked()
                                    {
                                        add_instance = params.pending.selected.map(|description| {
                                            params.next_instance_name.get_for(description)
                                        });
                                    }

                                    let selected_description_text =
                                        if let Some(description) = params.pending.selected {
                                            params
                                                .descriptions
                                                .get(description)
                                                .map(|n| n.as_str())
                                                .unwrap_or("")
                                        } else {
                                            ""
                                        };

                                    let mut selected_new_description = None;
                                    ComboBox::from_id_salt("choose_model_description")
                                        .selected_text(selected_description_text)
                                        .show_ui(ui, |ui| {
                                            let Ok(children) = params.children.get(site_entity)
                                            else {
                                                return;
                                            };
                                            for child in children {
                                                if let Ok(name) = params.descriptions.get(*child) {
                                                    ui.selectable_value(
                                                        &mut selected_new_description,
                                                        Some(*child),
                                                        name.as_str(),
                                                    );
                                                }
                                            }
                                        });

                                    if let Some(selected_new_description) = selected_new_description
                                    {
                                        params.pending.selected = Some(selected_new_description);
                                        add_instance = Some(
                                            params
                                                .next_instance_name
                                                .get_for(selected_new_description),
                                        );
                                    }
                                })
                                .response
                                .on_hover_text("Select Description");

                                ui.add_space(5.0);
                                ui.separator();

                                // Section for creating a new description
                                ui.label("New Model Description");
                                ui.add_space(2.0);
                                ui.horizontal(|ui| {
                                    ui.label("Name");
                                    ui.text_edit_singleline(&mut params.pending.name);
                                });

                                ui.add_space(10.0);
                                let default_file = params
                                    .current_workspace
                                    .root
                                    .map(|e| params.default_file.get(e).ok())
                                    .flatten();
                                if let Some(new_asset_source) = InspectAssetSourceComponent::new(
                                    &params.pending.source,
                                    &params.pending.recall_source,
                                    default_file,
                                )
                                .show(ui)
                                {
                                    params.pending.recall_source.remember(&new_asset_source);
                                    params.pending.source = new_asset_source;
                                }

                                ui.add_space(5.0);
                                if let Some(new_scale) =
                                    InspectScaleComponent::new(&params.pending.scale).show(ui)
                                {
                                    params.pending.scale = new_scale;
                                }

                                ui.add_space(5.0);
                                if ui
                                    .add(Button::image_and_text(
                                        params.icons.add.egui(),
                                        "Add Description",
                                    ))
                                    .clicked()
                                {
                                    let description = ModelDescriptionBundle {
                                        name: NameInSite(params.pending.name.clone()),
                                        source: ModelProperty(params.pending.source.clone()),
                                        is_static: ModelProperty(IsStatic::default()),
                                        scale: ModelProperty(params.pending.scale.clone()),
                                        ..Default::default()
                                    };

                                    let description_entity = params
                                        .commands
                                        .spawn(description)
                                        .insert(Category::ModelDescription)
                                        .insert(ChildOf(site_entity))
                                        .id();

                                    params.pending.selected = Some(description_entity);
                                    add_instance = Some(format!("{}_0", params.pending.name));
                                }

                                ui.add_space(10.0);
                            });
                    });

                    if let Some(new_instance_name) = add_instance {
                        if let Some(description) = params.pending.selected {
                            let instance = ModelInstance {
                                name: NameInSite(new_instance_name),
                                description: Affiliation(Some(description)),
                                ..Default::default()
                            };
                            params.object_placement.place_object_2d(instance);
                        }

                        ui.close_menu();
                    }
                })
                .response
                .on_hover_text("Model");
            }
            _ => {}
        }
    }
}

#[derive(SystemParam)]
pub struct GetNextInstanceName<'w, 's> {
    names: Query<'w, 's, &'static NameInSite>,
    members: Query<'w, 's, &'static Members>,
}

impl<'w, 's> GetNextInstanceName<'w, 's> {
    pub fn get_for(&self, description: Entity) -> String {
        let base_name = self
            .names
            .get(description)
            .map(|n| n.as_str())
            .unwrap_or("<unnamed>");

        let index = if let Ok(members) = self.members.get(description) {
            let mut index = 0;
            let mut changed = true;
            while changed {
                let test_name = format!("{base_name}_{index}");
                changed = false;
                for member in members.iter() {
                    if let Ok(name) = self.names.get(*member) {
                        if test_name == **name {
                            changed = true;
                            index += 1;
                            break;
                        }
                    }
                }
            }
            index
        } else {
            0
        };

        format!("{base_name}_{index}")
    }
}

#[derive(Default)]
pub struct BrowseFuelTogglePlugin {}

impl Plugin for BrowseFuelTogglePlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<BrowseFuelToggle>::new());
    }
}

#[derive(SystemParam)]
pub struct BrowseFuelToggle<'w> {
    asset_gallery: Option<ResMut<'w, AssetGalleryStatus>>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w> WidgetSystem<Tile> for BrowseFuelToggle<'w> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        if !matches!(params.app_state.get(), AppState::SiteEditor) {
            return;
        }

        let enabled = params.asset_gallery.is_some();
        let toggled_on = params
            .asset_gallery
            .as_ref()
            .is_some_and(|gallery| gallery.show);
        let tooltip = if !enabled {
            "Fuel asset browser is not available"
        } else if toggled_on {
            "Close fuel asset browser"
        } else {
            "Open fuel asset browser"
        };

        if ui
            .add_enabled(enabled, Button::new("🌐").selected(toggled_on))
            .on_hover_text(tooltip)
            .clicked()
        {
            if let Some(gallery) = &mut params.asset_gallery {
                gallery.show = !gallery.show;
            }
        }
    }
}

/// Helper funtion to display the button name on hover
fn button_clicked(ui: &mut Ui, icon: &str, tooltip: &str) -> bool {
    ui.add(Button::new(icon)).on_hover_text(tooltip).clicked()
}
