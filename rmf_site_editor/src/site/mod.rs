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

pub mod display_color;
pub use display_color::*;

pub mod door;
pub use door::*;

pub mod drawing;
pub use drawing::*;

pub mod floor;
pub use floor::*;

pub mod lane;
pub use lane::*;

pub mod level;
pub use level::*;

pub mod lift;
pub use lift::*;

pub mod light;
pub use light::*;

pub mod load;
pub use load::*;

pub mod location;
pub use location::*;

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

pub mod sdf;
pub use sdf::*;

pub mod save;
pub use save::*;

pub mod site;
pub use site::*;

pub mod screenspace_selection;
pub use screenspace_selection::*;

pub mod util;
pub use util::*;

pub mod wall;
pub use wall::*;

pub mod offscreen_render_tests;
pub use offscreen_render_tests::*;

pub mod camera_capture;
pub use camera_capture::*;

use crate::recency::{RecencyRank, RecencyRankingPlugin};
pub use rmf_site_format::*;

use bevy::{prelude::*, render::view::visibility::VisibilitySystems, transform::TransformSystem};

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
pub enum SiteUpdateStage {
    /// We need a custom stage for assigning orphan elements because the
    /// commands from CoreStage::Update need to flush before the AssignOrphan
    /// systems are run, and the AssignOrphan commands need to flush before the
    /// PostUpdate systems are run.
    AssignOrphans,
    /// Use a custom stage for deletions to make sure that all commands are
    /// flushed before and after deleting things.
    Deletion,
}

pub struct SitePlugin;

impl Plugin for SitePlugin {
    fn build(&self, app: &mut App) {
        app.add_state(SiteState::Off)
            .add_stage_after(
                CoreStage::Update,
                SiteUpdateStage::AssignOrphans,
                SystemStage::parallel(),
            )
            .add_state_to_stage(CoreStage::First, SiteState::Off)
            .add_state_to_stage(CoreStage::PreUpdate, SiteState::Off)
            .add_state_to_stage(SiteUpdateStage::AssignOrphans, SiteState::Off)
            .add_state_to_stage(CoreStage::PostUpdate, SiteState::Off)
            .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
            .insert_resource(FloorVisibility::default())
            .init_resource::<SiteAssets>()
            .init_resource::<LoadingDrawings>()
            .init_resource::<CurrentLevel>()
            .init_resource::<PhysicalLightToggle>()
            .add_event::<LoadSite>()
            .add_event::<ImportNavGraphs>()
            .add_event::<ChangeCurrentSite>()
            .add_event::<SaveSite>()
            .add_event::<SaveNavGraphs>()
            .add_event::<ToggleLiftDoorAvailability>()
            .add_event::<ExportLights>()
            .add_event::<ConsiderAssociatedGraph>()
            .add_event::<ConsiderLocationTag>()
            .add_plugin(ChangePlugin::<AssociatedGraphs<Entity>>::default())
            .add_plugin(RecallPlugin::<RecallAssociatedGraphs<Entity>>::default())
            .add_plugin(ChangePlugin::<Motion>::default())
            .add_plugin(RecallPlugin::<RecallMotion>::default())
            .add_plugin(ChangePlugin::<ReverseLane>::default())
            .add_plugin(RecallPlugin::<RecallReverseLane>::default())
            .add_plugin(ChangePlugin::<NameInSite>::default())
            .add_plugin(ChangePlugin::<NameInWorkcell>::default())
            .add_plugin(ChangePlugin::<Pose>::default())
            .add_plugin(ChangePlugin::<Scale>::default())
            .add_plugin(ChangePlugin::<MeshConstraint<Entity>>::default())
            .add_plugin(ChangePlugin::<Label>::default())
            .add_plugin(RecallPlugin::<RecallLabel>::default())
            .add_plugin(ChangePlugin::<DoorType>::default())
            .add_plugin(RecallPlugin::<RecallDoorType>::default())
            .add_plugin(ChangePlugin::<LevelProperties>::default())
            .add_plugin(ChangePlugin::<LiftCabin<Entity>>::default())
            .add_plugin(RecallPlugin::<RecallLiftCabin<Entity>>::default())
            .add_plugin(ChangePlugin::<AssetSource>::default())
            .add_plugin(RecallPlugin::<RecallAssetSource>::default())
            .add_plugin(ChangePlugin::<MeshPrimitive>::default())
            .add_plugin(RecallPlugin::<RecallMeshPrimitive>::default())
            .add_plugin(ChangePlugin::<PixelsPerMeter>::default())
            .add_plugin(ChangePlugin::<PhysicalCameraProperties>::default())
            .add_plugin(ChangePlugin::<LightKind>::default())
            .add_plugin(RecallPlugin::<RecallLightKind>::default())
            .add_plugin(ChangePlugin::<DisplayColor>::default())
            .add_plugin(ChangePlugin::<LocationTags>::default())
            .add_plugin(RecallPlugin::<RecallLocationTags>::default())
            .add_plugin(ChangePlugin::<Visibility>::default())
            .add_plugin(ChangePlugin::<FloorVisibility>::default())
            .add_plugin(RecencyRankingPlugin::<NavGraphMarker>::default())
            .add_plugin(RecencyRankingPlugin::<FloorMarker>::default())
            .add_plugin(RecencyRankingPlugin::<DrawingMarker>::default())
            .add_plugin(DeletionPlugin)
            .add_plugin(ImageCopyPlugin)
            .add_system(load_site)
            .add_system(import_nav_graph)
            .add_system(resize_notificator)
            .add_system_to_stage(CoreStage::PostUpdate, image_saver)
            .init_resource::<ColorEntityMap>()
            .add_system(screenspace_selection_system)
            .add_system_set_to_stage(
                CoreStage::PreUpdate,
                SystemSet::on_update(SiteState::Display)
                    .after(SiteUpdateLabel::ProcessChanges)
                    .with_system(update_lift_cabin)
                    .with_system(update_lift_edge)
                    .with_system(update_model_tentative_formats)
                    .with_system(update_material_for_display_color),
            )
            .add_system_set(
                SystemSet::on_update(SiteState::Display)
                    .with_system(save_site)
                    .with_system(save_nav_graphs)
                    .with_system(change_site.before(load_site)),
            )
            .add_system_set_to_stage(
                SiteUpdateStage::AssignOrphans,
                SystemSet::on_update(SiteState::Display)
                    .with_system(assign_orphan_anchors_to_parent)
                    .with_system(assign_orphan_levels_to_site)
                    .with_system(assign_orphan_nav_elements_to_site)
                    .with_system(assign_orphan_elements_to_level::<DoorMarker>)
                    .with_system(assign_orphan_elements_to_level::<DrawingMarker>)
                    .with_system(assign_orphan_elements_to_level::<FloorMarker>)
                    .with_system(assign_orphan_elements_to_level::<LightKind>)
                    .with_system(assign_orphan_elements_to_level::<ModelMarker>)
                    .with_system(assign_orphan_elements_to_level::<PhysicalCameraProperties>)
                    .with_system(assign_orphan_elements_to_level::<WallMarker>)
                    .with_system(add_tags_to_lift)
                    .with_system(add_material_for_display_colors)
                    .with_system(add_physical_lights),
            )
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::on_update(SiteState::Display)
                    .before(TransformSystem::TransformPropagate)
                    .after(VisibilitySystems::VisibilityPropagate)
                    .with_system(update_anchor_transforms)
                    .with_system(add_door_visuals)
                    .with_system(update_changed_door)
                    .with_system(update_door_for_moved_anchors)
                    .with_system(add_floor_visuals)
                    .with_system(update_changed_floor)
                    .with_system(update_floor_for_moved_anchors)
                    .with_system(update_floor_visibility)
                    .with_system(add_lane_visuals)
                    .with_system(add_location_visuals)
                    .with_system(update_level_visibility)
                    .with_system(update_changed_lane)
                    .with_system(update_lane_for_moved_anchor)
                    .with_system(remove_association_for_deleted_graphs)
                    .with_system(
                        update_visibility_for_lanes.after(remove_association_for_deleted_graphs),
                    )
                    .with_system(
                        update_visibility_for_locations
                            .after(remove_association_for_deleted_graphs),
                    )
                    .with_system(update_changed_location)
                    .with_system(update_location_for_moved_anchors)
                    .with_system(handle_consider_associated_graph)
                    .with_system(handle_consider_location_tag)
                    .with_system(update_lift_for_moved_anchors)
                    .with_system(update_lift_door_availability)
                    .with_system(update_physical_lights)
                    .with_system(toggle_physical_lights)
                    .with_system(add_measurement_visuals)
                    .with_system(update_changed_measurement)
                    .with_system(update_measurement_for_moved_anchors)
                    .with_system(update_model_scenes)
                    .with_system(handle_new_sdf_roots)
                    .with_system(update_model_scales)
                    .with_system(make_models_selectable)
                    .with_system(handle_new_mesh_primitives)
                    .with_system(add_drawing_visuals)
                    .with_system(handle_loaded_drawing)
                    .with_system(update_drawing_visuals)
                    .with_system(update_drawing_rank)
                    .with_system(update_drawing_pixels_per_meter)
                    .with_system(add_physical_camera_visuals)
                    .with_system(add_wall_visual)
                    .with_system(update_wall_edge)
                    .with_system(update_wall_for_moved_anchors)
                    .with_system(update_transforms_for_changed_poses)
                    .with_system(export_lights),
            );
    }
}
