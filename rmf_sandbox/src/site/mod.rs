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

pub mod anchor;
pub use anchor::*;

pub mod assets;
pub use assets::*;

pub mod door;
pub use door::*;

pub mod floor;
pub use floor::*;

pub mod lane;
pub use lane::*;

pub mod lift;
pub use lift::*;

pub mod light;
pub use light::*;

pub mod load;
pub use load::*;

pub mod measurement;
pub use measurement::*;

pub mod model;
pub use model::*;

pub mod physical_camera;
pub use physical_camera::*;

pub mod save;
pub use save::*;

pub mod site;
pub use site::*;

pub mod util;
pub use util::*;

pub mod wall;
pub use wall::*;

use bevy::prelude::*;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SiteState {
    Off,
    Display,
}

pub struct SitePlugin;

impl Plugin for SitePlugin {
    fn build(&self, app: &mut App) {
        app
            .add_state(SiteState::Off)
            .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
            .init_resource::<SiteAssets>()
            .init_resource::<SpawnedModels>()
            .init_resource::<LoadingModels>()
            .init_resource::<OpenSites>()
            .init_resource::<CurrentSite>()
            .init_resource::<CurrentLevel>()
            .init_resource::<CachedLevels>()
            .add_event::<LoadSite>()
            .add_event::<ChangeCurrentSite>()
            .add_event::<SaveSite>()
            .add_system(load_site)
            .add_system_set(
                SystemSet::on_enter(SiteState::Display)
                    .with_system(site_display_on)
            )
            .add_system_set(
                SystemSet::on_exit(SiteState::Display)
                    .with_system(site_display_off)
            )
            .add_system_set(
                SystemSet::on_update(SiteState::Display)
                    .with_system(save_site.exclusive_system())
                    .with_system(change_site)
                    .with_system(add_door_visuals)
                    .with_system(update_changed_door)
                    .with_system(update_door_for_changed_anchor)
                    .with_system(add_floor_visuals)
                    .with_system(update_changed_floor)
                    .with_system(update_floor_for_changed_anchor)
                    .with_system(add_lane_visuals)
                    .with_system(update_changed_lane)
                    .with_system(update_lane_for_changed_anchor)
                    .with_system(update_lanes_for_changed_level)
                    .with_system(add_lift_visuals)
                    .with_system(update_changed_lift)
                    .with_system(update_lift_for_changed_anchor)
                    .with_system(add_physical_lights)
                    .with_system(add_measurement_visuals)
                    .with_system(update_changed_measurement)
                    .with_system(update_measurement_for_changed_anchor)
                    .with_system(update_models)
                    .with_system(make_models_selectable)
                    .with_system(add_physical_camera_visuals)
                    .with_system(update_changed_physical_camera_visuals)
                    .with_system(add_wall_visual)
                    .with_system(update_changed_wall)
                    .with_system(update_wall_for_changed_anchor)
            );
    }
}
