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

pub mod change_plugin;
pub use change_plugin::*;

pub mod deletion;
pub use deletion::*;

pub mod door;
pub use door::*;

pub mod drawing;
pub use drawing::*;

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

pub mod nav_graph;
pub use nav_graph::*;

pub mod path;
pub use path::*;

pub mod physical_camera;
pub use physical_camera::*;

pub mod pose;
pub use pose::*;

pub mod recall_plugin;
pub use recall_plugin::RecallPlugin;

pub mod save;
pub use save::*;

pub mod site;
pub use site::*;

pub mod util;
pub use util::*;

pub mod wall;
pub use wall::*;

use rmf_site_format::*;

use bevy::{prelude::*, render::view::visibility::VisibilitySystems, transform::TransformSystem};

/// The Category component is added to site entities so they can easily express
/// what kind of thing they are, e.g. Anchor, Lane, Model, etc. This should be
/// set by the respective site system that decorates its entities with
/// components, e.g. add_door_visuals, add_lane_visuals, etc.
///
/// The information in this component is intended to be presented to humans to
/// read, and is not meant to be a key for identifying the type of an entity.
#[derive(Debug, Clone, Hash, PartialEq, Eq, Component, Deref, DerefMut)]
pub struct Category(pub String);

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum SiteState {
    Off,
    Display,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemLabel)]
pub enum SiteUpdateLabel {
    ProcessChanges,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum SiteCustomStage {
    /// We need a custom stage for assigning orphan elements because the
    /// commands from CoreStage::Update need to flush before the AssignOrphan
    /// systems are run, and the AssignOrphan commands need to flush before the
    /// PostUpdate systems are run.
    AssignOrphans,
}

pub struct SitePlugin;

impl Plugin for SitePlugin {
    fn build(&self, app: &mut App) {
        app.add_state(SiteState::Off)
            .add_stage_after(
                CoreStage::Update,
                SiteCustomStage::AssignOrphans,
                SystemStage::parallel(),
            )
            .add_state_to_stage(CoreStage::First, SiteState::Off)
            .add_state_to_stage(CoreStage::PreUpdate, SiteState::Off)
            .add_state_to_stage(SiteCustomStage::AssignOrphans, SiteState::Off)
            .add_state_to_stage(CoreStage::PostUpdate, SiteState::Off)
            .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
            .init_resource::<SiteAssets>()
            .init_resource::<SpawnedModels>()
            .init_resource::<LoadingModels>()
            .init_resource::<LoadingDrawings>()
            .init_resource::<OpenSites>()
            .init_resource::<CurrentSite>()
            .init_resource::<CurrentLevel>()
            .init_resource::<CachedLevels>()
            .init_resource::<SelectedNavGraph>()
            .add_event::<LoadSite>()
            .add_event::<ChangeCurrentSite>()
            .add_event::<SaveSite>()
            .add_plugin(ChangePlugin::<Motion>::default())
            .add_plugin(RecallPlugin::<RecallMotion>::default())
            .add_plugin(ChangePlugin::<ReverseLane>::default())
            .add_plugin(RecallPlugin::<RecallReverseLane>::default())
            .add_plugin(ChangePlugin::<NameInSite>::default())
            .add_plugin(ChangePlugin::<Pose>::default())
            .add_plugin(ChangePlugin::<Kind>::default())
            .add_plugin(RecallPlugin::<RecallKind>::default())
            .add_plugin(ChangePlugin::<Label>::default())
            .add_plugin(RecallPlugin::<RecallLabel>::default())
            .add_plugin(ChangePlugin::<DoorType>::default())
            .add_plugin(RecallPlugin::<RecallDoorType>::default())
            .add_plugin(ChangePlugin::<AssetSource>::default())
            .add_plugin(RecallPlugin::<RecallAssetSource>::default())
            .add_plugin(ChangePlugin::<PixelsPerMeter>::default())
            .add_plugin(DeletionPlugin)
            .add_system(load_site)
            .add_system_set(SystemSet::on_enter(SiteState::Display).with_system(site_display_on))
            .add_system_set(SystemSet::on_exit(SiteState::Display).with_system(site_display_off))
            .add_system_set(
                SystemSet::on_update(SiteState::Display)
                    .with_system(save_site.exclusive_system())
                    .with_system(change_site),
            )
            .add_system_set_to_stage(
                SiteCustomStage::AssignOrphans,
                SystemSet::on_update(SiteState::Display)
                    .with_system(assign_orphan_anchors_to_parent)
                    .with_system(assign_orphans_to_nav_graph),
            )
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::on_update(SiteState::Display)
                    .before(TransformSystem::TransformPropagate)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .with_system(add_door_visuals)
                    .with_system(update_changed_door)
                    .with_system(update_door_for_changed_anchor)
                    .with_system(add_floor_visuals)
                    .with_system(update_changed_floor)
                    .with_system(update_floor_for_changed_anchor)
                    .with_system(add_lane_visuals)
                    .with_system(update_changed_lane)
                    .with_system(update_lane_for_moved_anchor)
                    .with_system(update_visibility_for_lanes)
                    .with_system(add_lift_visuals)
                    .with_system(update_changed_lift)
                    .with_system(update_lift_for_changed_anchor)
                    .with_system(add_physical_lights)
                    .with_system(add_measurement_visuals)
                    .with_system(update_changed_measurement)
                    .with_system(update_measurement_for_changed_anchor)
                    .with_system(update_model_scenes)
                    .with_system(make_models_selectable)
                    .with_system(add_drawing_visuals)
                    .with_system(handle_loaded_drawing)
                    .with_system(update_drawing_asset_source)
                    .with_system(update_drawing_pixels_per_meter)
                    .with_system(add_physical_camera_visuals)
                    .with_system(add_wall_visual)
                    .with_system(update_changed_wall)
                    .with_system(update_wall_for_changed_anchor)
                    .with_system(update_transforms_for_changed_poses),
            );
    }
}
