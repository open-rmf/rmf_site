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

use crate::{interaction::AnchorSelection, site::CurrentLevel, AppState};
use bevy::prelude::*;
use rmf_site_egui::*;

pub(crate) struct ZonesPlugin;

#[derive(Resource)]
struct ZoneMenu(Entity);

impl Plugin for ZonesPlugin {
    fn build(&self, app: &mut App) {
        let tool = app.world().resource::<ToolMenu>().get();
        let item = app
            .world_mut()
            .spawn((MenuItem::Text("Add Zone".into()), ChildOf(tool)))
            .id();
        app.insert_resource(ZoneMenu(item))
            .add_systems(Update, draw_zone.run_if(in_state(AppState::SiteEditor)));
    }
}

fn draw_zone(
    mut events: EventReader<MenuEvent>,
    menu: Res<ZoneMenu>,
    level: Res<CurrentLevel>,
    mut selection: AnchorSelection,
    mouse: Res<ButtonInput<MouseButton>>,
    mut pending: Local<bool>,
) {
    // Don't let the menu click place the first vertex.
    if *pending && !mouse.pressed(MouseButton::Left) && !mouse.just_released(MouseButton::Left) {
        *pending = false;
        if level.0.is_some() {
            selection.create_zone();
        }
    }
    for event in events.read() {
        if event.clicked() && event.source() == menu.0 {
            *pending = true;
        }
    }
}
