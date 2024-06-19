/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use crate::interaction::{CategoryVisibility, SetCategoryVisibility};
use crate::site::{
    CollisionMeshMarker, DoorMarker, FiducialMarker, FloorMarker, LaneMarker, LiftCabin,
    LiftCabinDoorMarker, LocationTags, MeasurementMarker, VisualMeshMarker, WallMarker,
};
use crate::{
    widgets::menu_bar::{MenuEvent, MenuItem, MenuVisualizationStates, ViewMenu},
    workcell::WorkcellVisualizationMarker,
    AppState, FloorGrid,
};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use std::collections::HashSet;

#[derive(SystemParam)]
struct VisibilityEvents<'w> {
    doors: EventWriter<'w, SetCategoryVisibility<DoorMarker>>,
    floors: EventWriter<'w, SetCategoryVisibility<FloorMarker>>,
    lanes: EventWriter<'w, SetCategoryVisibility<LaneMarker>>,
    lift_cabins: EventWriter<'w, SetCategoryVisibility<LiftCabin<Entity>>>,
    lift_cabin_doors: EventWriter<'w, SetCategoryVisibility<LiftCabinDoorMarker>>,
    locations: EventWriter<'w, SetCategoryVisibility<LocationTags>>,
    fiducials: EventWriter<'w, SetCategoryVisibility<FiducialMarker>>,
    measurements: EventWriter<'w, SetCategoryVisibility<MeasurementMarker>>,
    walls: EventWriter<'w, SetCategoryVisibility<WallMarker>>,
    visuals: EventWriter<'w, SetCategoryVisibility<VisualMeshMarker>>,
    collisions: EventWriter<'w, SetCategoryVisibility<CollisionMeshMarker>>,
    origin_axis: EventWriter<'w, SetCategoryVisibility<WorkcellVisualizationMarker>>,
}

#[derive(Default)]
pub struct ViewMenuPlugin;

#[derive(Resource)]
pub struct ViewMenuItems {
    doors: Entity,
    floors: Entity,
    lanes: Entity,
    lifts: Entity,
    locations: Entity,
    fiducials: Entity,
    measurements: Entity,
    collisions: Entity,
    visuals: Entity,
    walls: Entity,
    origin_axis: Entity,
    floor_grid: Entity,
}

impl FromWorld for ViewMenuItems {
    fn from_world(world: &mut World) -> Self {
        let site_states = HashSet::from([
            AppState::SiteEditor,
            AppState::SiteDrawingEditor,
            AppState::SiteVisualizer,
        ]);
        let workcell_states = HashSet::from([AppState::WorkcellEditor]);
        let mut active_states = site_states.clone();
        active_states.insert(AppState::WorkcellEditor);
        let view_header = world.resource::<ViewMenu>().get();
        let default_visibility = world.resource::<CategoryVisibility<DoorMarker>>();
        let doors = world
            .spawn(MenuItem::CheckBox(
                "Doors".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(site_states.clone()))
            .set_parent(view_header)
            .id();
        let default_visibility = world.resource::<CategoryVisibility<FloorMarker>>();
        let floors = world
            .spawn(MenuItem::CheckBox(
                "Floors".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(site_states.clone()))
            .set_parent(view_header)
            .id();
        let default_visibility = world.resource::<CategoryVisibility<LaneMarker>>();
        let lanes = world
            .spawn(MenuItem::CheckBox(
                "Lanes".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(site_states.clone()))
            .set_parent(view_header)
            .id();
        let default_visibility = world.resource::<CategoryVisibility<LiftCabin<Entity>>>();
        let lifts = world
            .spawn(MenuItem::CheckBox(
                "Lifts".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(site_states.clone()))
            .set_parent(view_header)
            .id();
        let default_visibility = world.resource::<CategoryVisibility<LocationTags>>();
        let locations = world
            .spawn(MenuItem::CheckBox(
                "Locations".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(site_states.clone()))
            .set_parent(view_header)
            .id();
        let default_visibility = world.resource::<CategoryVisibility<FiducialMarker>>();
        let fiducials = world
            .spawn(MenuItem::CheckBox(
                "Fiducials".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(site_states.clone()))
            .set_parent(view_header)
            .id();
        let default_visibility = world.resource::<CategoryVisibility<MeasurementMarker>>();
        let measurements = world
            .spawn(MenuItem::CheckBox(
                "Measurements".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(site_states.clone()))
            .set_parent(view_header)
            .id();
        let default_visibility = world.resource::<CategoryVisibility<CollisionMeshMarker>>();
        let collisions = world
            .spawn(MenuItem::CheckBox(
                "Collision meshes".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(active_states.clone()))
            .set_parent(view_header)
            .id();
        let default_visibility = world.resource::<CategoryVisibility<VisualMeshMarker>>();
        let visuals = world
            .spawn(MenuItem::CheckBox(
                "Visual meshes".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(active_states.clone()))
            .set_parent(view_header)
            .id();
        let default_visibility = world.resource::<CategoryVisibility<WallMarker>>();
        let walls = world
            .spawn(MenuItem::CheckBox(
                "Walls".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(site_states))
            .set_parent(view_header)
            .id();
        let default_visibility =
            world.resource::<CategoryVisibility<WorkcellVisualizationMarker>>();
        let origin_axis = world
            .spawn(MenuItem::CheckBox(
                "Reference axis".to_string(),
                default_visibility.0,
            ))
            .insert(MenuVisualizationStates(workcell_states))
            .set_parent(view_header)
            .id();
        let floor_grid = world
            .spawn(MenuItem::CheckBox(
                "Floor grid".to_string(),
                true,
            ))
            .insert(MenuVisualizationStates(active_states))
            .set_parent(view_header)
            .id();

        ViewMenuItems {
            doors,
            floors,
            lanes,
            lifts,
            locations,
            fiducials,
            measurements,
            collisions,
            visuals,
            walls,
            origin_axis,
            floor_grid,
        }
    }
}

fn handle_view_menu_events(
    mut menu_events: EventReader<MenuEvent>,
    view_menu: Res<ViewMenuItems>,
    mut menu_items: Query<&mut MenuItem>,
    mut events: VisibilityEvents,
    floor_grid: Res<FloorGrid>,
    mut visibility: Query<&mut Visibility>,
) {
    let mut toggle = |entity| {
        let mut menu = menu_items.get_mut(entity).unwrap();
        let value = menu.checkbox_value_mut().unwrap();
        *value = !*value;
        *value
    };
    for event in menu_events.read() {
        if event.clicked() && event.source() == view_menu.doors {
            events.doors.send(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.floors {
            events.floors.send(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.lanes {
            events.lanes.send(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.lifts {
            let value = toggle(event.source());
            events.lift_cabins.send(value.into());
            events.lift_cabin_doors.send(value.into());
        } else if event.clicked() && event.source() == view_menu.locations {
            events.locations.send(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.fiducials {
            events.fiducials.send(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.measurements {
            events.measurements.send(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.collisions {
            events.collisions.send(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.visuals {
            events.visuals.send(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.walls {
            events.walls.send(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.origin_axis {
            events.origin_axis.send(toggle(event.source()).into());
        } else if event.clicked() && event.source() == view_menu.floor_grid {
            let floor_grid_visible = toggle(event.source());
            if let Ok(mut vis) = visibility.get_mut(floor_grid.get()) {
                if floor_grid_visible {
                    *vis = Visibility::Visible;
                } else {
                    *vis = Visibility::Hidden;
                }
            }
        }
    }
}

impl Plugin for ViewMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ViewMenuItems>()
            .add_systems(Update, handle_view_menu_events);
    }
}
