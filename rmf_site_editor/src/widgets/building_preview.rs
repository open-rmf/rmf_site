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

use crate::{
    widgets::prelude::*,
    AppState,
};
use bevy::prelude::*;
use bevy_egui::egui::{Button, Ui};

#[derive(Default)]
pub struct BuildingPreviewPlugin {

}

impl Plugin for BuildingPreviewPlugin {
    fn build(&self, app: &mut App) {
        let widget = Widget::new::<BuildingPreview>(&mut app.world);
        let properties_panel = app.world.resource::<PropertiesPanel>().id;
        app.world.spawn(widget).set_parent(properties_panel);
    }
}

#[derive(SystemParam)]
pub struct BuildingPreview<'w> {
    next_app_state: ResMut<'w, NextState<AppState>>,
}

impl<'w> WidgetSystem<Tile> for BuildingPreview<'w> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if ui.add(Button::new("Building preview")).clicked() {
            params.next_app_state.set(AppState::SiteVisualizer);
        }
    }
}
