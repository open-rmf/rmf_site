/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use super::*;
use crate::prelude::SystemState;
use bevy::{
    ecs::system::SystemParam,
    prelude::*,
};
use bevy_egui::egui::Ui;
use rmf_site_egui::{Tile, WidgetSystem};

#[derive(SystemParam)]
pub struct MapfConfigWidget<'w> {
    negotiation_debug: ResMut<'w, NegotiationDebugData>,
}

impl<'w> WidgetSystem<Tile> for MapfConfigWidget<'w> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        ui.separator();

        // Toggle debug panel
        ui.horizontal(|ui| {
            ui.label("MAPF Debug Panel");
            ui.checkbox(&mut params.negotiation_debug.show_debug_panel, "Enabled");
        });
    }
}
