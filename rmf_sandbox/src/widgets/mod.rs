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
    site::{SiteState, SiteUpdateLabel},
    interaction::PickingBlockers,
};
use bevy::{
    prelude::*,
};
use bevy_egui::{
    egui, EguiContext,
};

pub mod inspector;
use inspector::{InspectorWidget, InspectorParams};

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum UiUpdateLabel {
    DrawUi,
    BlockPicking,
}

#[derive(Default)]
pub struct StandardUiLayout;

impl Plugin for StandardUiLayout {
    fn build(&self, app: &mut App) {
        app
            .add_system_set(
                SystemSet::on_update(SiteState::Display)
                    .after(SiteUpdateLabel::AllSystems)
                    .with_system(
                        standard_ui_layout.label(UiUpdateLabel::DrawUi)
                    )
                    .with_system(
                        ui_picking_blocker
                        .after(UiUpdateLabel::DrawUi)
                        .label(UiUpdateLabel::BlockPicking)
                    )
            );
    }
}

fn standard_ui_layout(
    mut egui_context: ResMut<EguiContext>,
    mut inspector_params: InspectorParams,
) {
    egui::SidePanel::right("inspector_panel")
        .resizable(true)
        .default_width(250.)
        .show(egui_context.ctx_mut(), |ui| {
            ui.vertical(|ui| {
                ui.add(InspectorWidget{params: &mut inspector_params})
            })
        });
}

fn ui_picking_blocker(
    mut egui_context: ResMut<EguiContext>,
    mut picking_blocker: Option<ResMut<PickingBlockers>>,
) {
    if let Some(picking_blocker) = &mut picking_blocker {
        let egui_context = egui_context.ctx_mut();
        picking_blocker.ui = egui_context.wants_pointer_input()
            || egui_context.wants_keyboard_input()
            || egui_context.is_pointer_over_area();
    }
}
