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

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use rmf_site_egui::{MenuEvent, MenuItem, ViewMenu};

use rmf_site_editor::{
    interaction::{CategoryVisibility, SetCategoryVisibility},
    AppState,
};

use super::local_paths::LocalPathMarker;
use super::planned_paths::{PathDependencyMarker, PlannedPathMarker};
use super::safe_zones::LiveSafeZoneMarker;

#[derive(SystemParam)]
struct LiveVisibilityEvents<'w> {
    planned_paths: EventWriter<'w, SetCategoryVisibility<PlannedPathMarker>>,
    path_dependencies: EventWriter<'w, SetCategoryVisibility<PathDependencyMarker>>,
    local_paths: EventWriter<'w, SetCategoryVisibility<LocalPathMarker>>,
    safe_zones: EventWriter<'w, SetCategoryVisibility<LiveSafeZoneMarker>>,
}

#[derive(Default)]
pub struct LiveViewMenuPlugin;

#[derive(Resource)]
struct LiveViewMenuItems {
    planned_paths: Entity,
    path_dependencies: Entity,
    local_paths: Entity,
    safe_zones: Entity,
}

impl FromWorld for LiveViewMenuItems {
    fn from_world(world: &mut World) -> Self {
        let default_visibility = world.resource::<CategoryVisibility<PlannedPathMarker>>();
        let planned_paths = world
            .spawn(MenuItem::CheckBox(
                "Planned Paths".to_string(),
                default_visibility.0,
            ))
            .id();
        let default_visibility = world.resource::<CategoryVisibility<PathDependencyMarker>>();
        let path_dependencies = world
            .spawn(MenuItem::CheckBox(
                "Path Dependencies".to_string(),
                default_visibility.0,
            ))
            .id();
        let default_visibility = world.resource::<CategoryVisibility<LocalPathMarker>>();
        let local_paths = world
            .spawn(MenuItem::CheckBox(
                "Local Paths".to_string(),
                default_visibility.0,
            ))
            .id();
        let default_visibility = world.resource::<CategoryVisibility<LiveSafeZoneMarker>>();
        let safe_zones = world
            .spawn(MenuItem::CheckBox(
                "Safe Zones".to_string(),
                default_visibility.0,
            ))
            .id();

        LiveViewMenuItems {
            planned_paths,
            path_dependencies,
            local_paths,
            safe_zones,
        }
    }
}

fn show_live_view_menu_items(
    mut commands: Commands,
    view_menu: Res<ViewMenu>,
    items: Res<LiveViewMenuItems>,
) {
    commands.entity(view_menu.get()).add_children(&[
        items.planned_paths,
        items.path_dependencies,
        items.local_paths,
        items.safe_zones,
    ]);
}

fn hide_live_view_menu_items(
    mut commands: Commands,
    view_menu: Res<ViewMenu>,
    items: Res<LiveViewMenuItems>,
) {
    commands.entity(view_menu.get()).remove_children(&[
        items.planned_paths,
        items.path_dependencies,
        items.local_paths,
        items.safe_zones,
    ]);
}

fn handle_live_view_menu_events(
    mut menu_events: EventReader<MenuEvent>,
    view_menu: Res<LiveViewMenuItems>,
    mut menu_items: Query<&mut MenuItem>,
    mut events: LiveVisibilityEvents,
) {
    let mut toggle = |entity| {
        let mut menu = menu_items.get_mut(entity).unwrap();
        let value = menu.checkbox_value_mut().unwrap();
        *value = !*value;
        *value
    };
    for event in menu_events.read() {
        if event.clicked() && event.source() == view_menu.planned_paths {
            events.planned_paths.write(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.path_dependencies {
            events
                .path_dependencies
                .write(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.local_paths {
            events.local_paths.write(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.safe_zones {
            events.safe_zones.write(toggle(event.source()).into());
        }
    }
}

impl Plugin for LiveViewMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<LiveViewMenuItems>()
            .add_systems(OnEnter(AppState::SiteReadOnly), show_live_view_menu_items)
            .add_systems(OnExit(AppState::SiteReadOnly), hide_live_view_menu_items)
            .add_systems(Update, handle_live_view_menu_events);
    }
}
