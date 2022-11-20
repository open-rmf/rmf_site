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
    interaction::{ChangeMode, Hover, MoveTo, PickingBlockers, Select, SpawnPreview, HeadlightToggle},
    site::{Change, CurrentLevel, Delete, SiteState, SiteUpdateLabel, ToggleLiftDoorAvailability, PhysicalLightToggle},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::{
    egui::{self, CollapsingHeader},
    EguiContext,
};
use rmf_site_format::*;

pub mod inspector;
use inspector::{InspectorParams, InspectorWidget};

pub mod create;
use create::CreateWidget;

pub mod view_levels;
use view_levels::{LevelDisplay, LevelParams, ViewLevels};

pub mod view_lights;
use view_lights::*;

pub mod icons;
pub use icons::*;

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
            .init_resource::<LightDisplay>()
            .add_system_set(SystemSet::on_enter(SiteState::Display).with_system(init_ui_style))
            .add_system_set(
                SystemSet::on_update(SiteState::Display)
                    .with_system(standard_ui_layout.label(UiUpdateLabel::DrawUi)),
            );
    }
}

#[derive(SystemParam)]
pub struct ChangeEvents<'w, 's> {
    pub lane_motion: EventWriter<'w, 's, Change<Motion>>,
    pub lane_reverse: EventWriter<'w, 's, Change<ReverseLane>>,
    pub name: EventWriter<'w, 's, Change<NameInSite>>,
    pub kind: EventWriter<'w, 's, Change<Kind>>,
    pub label: EventWriter<'w, 's, Change<Label>>,
    pub pose: EventWriter<'w, 's, Change<Pose>>,
    pub door: EventWriter<'w, 's, Change<DoorType>>,
    pub lift_cabin: EventWriter<'w, 's, Change<LiftCabin<Entity>>>,
    pub asset_source: EventWriter<'w, 's, Change<AssetSource>>,
    pub pixels_per_meter: EventWriter<'w, 's, Change<PixelsPerMeter>>,
    pub physical_camera_properties: EventWriter<'w, 's, Change<PhysicalCameraProperties>>,
    pub light: EventWriter<'w, 's, Change<LightKind>>,
}

/// We collect all the events into its own SystemParam because we are not
/// allowed to receive more than one EventWriter of a given type per system call
/// (for borrow-checker reasons). Bundling them all up into an AppEvents
/// parameter at least makes the EventWriters easy to pass around.
#[derive(SystemParam)]
pub struct AppEvents<'w, 's> {
    pub commands: Commands<'w, 's>,
    pub hover: ResMut<'w, Events<Hover>>,
    pub select: ResMut<'w, Events<Select>>,
    pub change: ChangeEvents<'w, 's>,
    pub move_to: EventWriter<'w, 's, MoveTo>,
    pub change_mode: ResMut<'w, Events<ChangeMode>>,
    pub current_level: ResMut<'w, CurrentLevel>,
    pub level_display: ResMut<'w, LevelDisplay>,
    pub light_display: ResMut<'w, LightDisplay>,
    pub change_level_props: EventWriter<'w, 's, Change<LevelProperties>>,
    pub toggle_door_levels: EventWriter<'w, 's, ToggleLiftDoorAvailability>,
    pub toggle_headlights: ResMut<'w, HeadlightToggle>,
    pub toggle_physical_lights: ResMut<'w, PhysicalLightToggle>,
    pub spawn_preview: EventWriter<'w, 's, SpawnPreview>,
    pub delete: EventWriter<'w, 's, Delete>,
}

fn standard_ui_layout(
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
    inspector_params: InspectorParams,
    levels: LevelParams,
    lights: LightParams,
    mut events: AppEvents,
) {
    egui::SidePanel::right("right_panel")
        .resizable(true)
        .show(egui_context.ctx_mut(), |ui| {
            let r = egui::ScrollArea::both()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    ui.vertical(|ui| {
                        CollapsingHeader::new("Levels")
                            .default_open(true)
                            .show(ui, |ui| {
                                ViewLevels::new(&levels, &mut events).show(ui);
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
        if events.hover.is_empty() {
            events.hover.send(Hover(None));
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
