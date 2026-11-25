/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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

use bevy::ecs::system::{SystemParam, SystemState};
use bevy::prelude::*;
use bevy_egui::egui;
use rmf_site_egui::WidgetSystem;

use rmf_site_editor::main_menu::MAIN_MENU_PADDING;

use super::live_state::LiveStreamState;

#[derive(SystemParam)]
pub struct StreamMenuSection<'w> {
    live_stream_state: ResMut<'w, LiveStreamState>,
}

impl<'w> WidgetSystem for StreamMenuSection<'w> {
    fn show(_: (), ui: &mut egui::Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);

        ui.heading("Visualize from Stream:");
        ui.add_space(MAIN_MENU_PADDING);

        ui.horizontal(|ui| {
            ui.label("ROSBridge WS URL:");
            ui.text_edit_singleline(&mut params.live_stream_state.url);
        });

        ui.horizontal(|ui| {
            ui.label("Site Data HTTP URL:");
            ui.text_edit_singleline(&mut params.live_stream_state.site_url);
        });

        ui.add_space(MAIN_MENU_PADDING * 0.5);

        let connection_initiated = params.live_stream_state.is_streaming();

        ui.vertical_centered_justified(|ui| {
            if connection_initiated {
                ui.label("Stream requested...");
            } else if ui.button("Connect").clicked() {
                params.live_stream_state.request_connection();
            }
        });
    }
}
