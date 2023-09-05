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

pub mod constraint;
pub use constraint::*;

pub mod deletion;
pub use deletion::*;

pub mod display_color;
pub use display_color::*;

pub mod drawing_editor;
pub use drawing_editor::*;

pub mod door;
pub use door::*;

pub mod drawing;
pub use drawing::*;

pub mod fiducial;
pub use fiducial::*;

pub mod floor;
pub use floor::*;

pub mod fuel_cache;
pub use fuel_cache::*;

pub mod georeference;
pub use georeference::*;

pub mod group;
pub use group::*;

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

pub mod site_visualizer;
pub use site_visualizer::*;

pub mod texture;
pub use texture::*;

pub mod util;
pub use util::*;

pub mod wall;
pub use wall::*;

use crate::recency::{RecencyRank, RecencyRankingPlugin};
pub use rmf_site_format::*;

use bevy::{prelude::*, render::view::visibility::VisibilitySystems, transform::TransformSystem};

#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq, States)]
pub enum SiteState {
    #[default]
    Off,
    Display,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum SiteUpdateSet {
    /// We need a custom stage for assigning orphan elements because the
    /// commands from CoreStage::Update need to flush before the AssignOrphan
    /// systems are run, and the AssignOrphan commands need to flush before the
    /// PostUpdate systems are run.
    AssignOrphans,
    /// Force a command flush
    AssignOrphansFlush,
    /// Use a custom stage for deletions to make sure that all commands are
    /// flushed before and after deleting things.
    Deletion,
    /// Force a command flush after deletion
    DeletionFlush,
    /// Placed between visibility and transform propagation, to avoid one frame delays
    BetweenVisibilityAndTransform,
    /// Flush the set above
    BetweenVisibilityAndTransformFlush,
    /// Used to force a command flush after the change plugin's process changes
    ProcessChanges,
    /// Flush the set above
    ProcessChangesFlush,
}

pub struct SitePlugin;

impl Plugin for SitePlugin {
    fn build(&self, app: &mut App) {
        app.add_state::<SiteState>()
            .configure_sets(
                (
                    PreUpdate,
                    SiteUpdateSet::ProcessChanges,
                    SiteUpdateSet::ProcessChangesFlush,
                ).chain()
            ).add_systems(SiteUpdateSet::ProcessChangesFlush, apply_deferred)
            .configure_sets(
                (
                    Update,
                    SiteUpdateSet::AssignOrphans,
                    SiteUpdateSet::AssignOrphansFlush,
                ).chain()
            ).add_systems(SiteUpdateSet::AssignOrphansFlush, apply_deferred)
            .configure_sets(
                (
                    VisibilitySystems::VisibilityPropagate,
                    SiteUpdateSet::BetweenVisibilityAndTransform,
                    SiteUpdateSet::BetweenVisibilityAndTransformFlush,
                    TransformSystem::TransformPropagate,
                ).chain()
            ).add_systems(SiteUpdateSet::BetweenVisibilityAndTransformFlush, apply_deferred)
            /*
            .add_state_to_stage(CoreStage::First, SiteState::Off)
            .add_state_to_stage(CoreStage::PreUpdate, SiteState::Off)
            .add_state_to_stage(SiteUpdateStage::AssignOrphans, SiteState::Off)
            .add_state_to_stage(CoreStage::PostUpdate, SiteState::Off)
            */
            .insert_resource(ClearColor(Color::rgb(0., 0., 0.)))
            .init_resource::<FuelClient>()
            .init_resource::<SiteAssets>()
            .init_resource::<CurrentLevel>()
            .init_resource::<PhysicalLightToggle>()
            .init_resource::<UpdateFuelCacheChannels>()
            .init_resource::<ModelTrashcan>()
            .add_event::<LoadSite>()
            .add_event::<ImportNavGraphs>()
            .add_event::<ChangeCurrentSite>()
            .add_event::<SaveSite>()
            .add_event::<SaveNavGraphs>()
            .add_event::<ToggleLiftDoorAvailability>()
            .add_event::<ExportLights>()
            .add_event::<ConsiderAssociatedGraph>()
            .add_event::<ConsiderLocationTag>()
            .add_event::<UpdateFuelCache>()
            .add_event::<SetFuelApiKey>()
            .add_event::<MergeGroups>()
            .add_plugin(ChangePlugin::<AssociatedGraphs<Entity>>::default())
            .add_plugin(RecallPlugin::<RecallAssociatedGraphs<Entity>>::default())
            .add_plugin(ChangePlugin::<Motion>::default())
            .add_plugin(RecallPlugin::<RecallMotion>::default())
            .add_plugin(ChangePlugin::<ReverseLane>::default())
            .add_plugin(RecallPlugin::<RecallReverseLane>::default())
            .add_plugin(ChangePlugin::<NameOfSite>::default())
            .add_plugin(ChangePlugin::<NameInSite>::default())
            .add_plugin(ChangePlugin::<NameInWorkcell>::default())
            .add_plugin(ChangePlugin::<Pose>::default())
            .add_plugin(ChangePlugin::<Scale>::default())
            .add_plugin(ChangePlugin::<MeshConstraint<Entity>>::default())
            .add_plugin(ChangePlugin::<Distance>::default())
            .add_plugin(ChangePlugin::<Texture>::default())
            .add_plugin(ChangePlugin::<Label>::default())
            .add_plugin(RecallPlugin::<RecallLabel>::default())
            .add_plugin(ChangePlugin::<DoorType>::default())
            .add_plugin(RecallPlugin::<RecallDoorType>::default())
            .add_plugin(ChangePlugin::<LevelElevation>::default())
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
            .add_plugin(ChangePlugin::<LayerVisibility>::default())
            .add_plugin(ChangePlugin::<GlobalFloorVisibility>::default())
            .add_plugin(ChangePlugin::<GlobalDrawingVisibility>::default())
            .add_plugin(ChangePlugin::<PreferredSemiTransparency>::default())
            .add_plugin(ChangePlugin::<Affiliation<Entity>>::default())
            .add_plugin(RecencyRankingPlugin::<NavGraphMarker>::default())
            .add_plugin(RecencyRankingPlugin::<FloorMarker>::default())
            .add_plugin(RecencyRankingPlugin::<DrawingMarker>::default())
            .add_plugin(DeletionPlugin)
            .add_plugin(DrawingEditorPlugin)
            .add_plugin(SiteVisualizerPlugin)
            .add_system(load_site)
            .add_system(import_nav_graph)
            .add_systems(
                PreUpdate, (
                    update_lift_cabin,
                    update_lift_edge,
                    update_model_tentative_formats,
                    update_drawing_pixels_per_meter,
                    update_drawing_children_to_pixel_coordinates,
                    fetch_image_for_texture,
                    detect_last_selected_texture::<FloorMarker>,
                    apply_last_selected_texture::<FloorMarker>.after(detect_last_selected_texture::<FloorMarker>),
                    detect_last_selected_texture::<WallMarker>,
                    apply_last_selected_texture::<WallMarker>.after(detect_last_selected_texture::<WallMarker>),
                    update_material_for_display_color,
                    ).after(SiteUpdateSet::ProcessChangesFlush).run_if(in_state(SiteState::Display))
            )
            .add_systems(
                Update, (
                    save_site,
                    save_nav_graphs,
                    change_site.before(load_site),
                ).run_if(in_state(SiteState::Display))
            )
            .add_systems(
                SiteUpdateSet::AssignOrphans, (
                    assign_orphan_anchors_to_parent,
                    assign_orphan_constraints_to_parent,
                    assign_orphan_levels_to_site,
                    assign_orphan_nav_elements_to_site,
                    assign_orphan_fiducials_to_parent,
                    assign_orphan_elements_to_level::<DoorMarker>,
                    assign_orphan_elements_to_level::<DrawingMarker>,
                    assign_orphan_elements_to_level::<FloorMarker>,
                    assign_orphan_elements_to_level::<LightKind>,
                    assign_orphan_elements_to_level::<ModelMarker>,
                    assign_orphan_elements_to_level::<PhysicalCameraProperties>,
                    assign_orphan_elements_to_level::<WallMarker>,
                    add_category_to_graphs,
                    add_tags_to_lift,
                    add_material_for_display_colors,
                    clear_model_trashcan,
                    add_physical_lights,
                    ).run_if(in_state(SiteState::Display))
            )
            .add_systems(
                SiteUpdateSet::BetweenVisibilityAndTransform, (
                    update_anchor_transforms,
                    add_door_visuals,
                    update_changed_door,
                    update_door_for_moved_anchors,
                    add_floor_visuals,
                    update_floors,
                    update_floors_for_moved_anchors,
                    update_floors,
                    update_floor_visibility,
                    update_drawing_visibility,
                    add_lane_visuals,
                    add_location_visuals,
                    add_fiducial_visuals,
                    add_constraint_visuals,
                    update_level_visibility,
                    update_changed_lane,
                    update_lane_for_moved_anchor,
                    remove_association_for_deleted_graphs,
                    add_unused_fiducial_tracker,
                    update_fiducial_usage_tracker,
                    update_visibility_for_lanes.after(remove_association_for_deleted_graphs),
                    update_visibility_for_locations.after(remove_association_for_deleted_graphs),
                    update_changed_location,
                    update_location_for_moved_anchors,
                    update_location_for_changed_location_tags,
                    update_changed_fiducial,
                    update_fiducial_for_moved_anchors,
                    handle_consider_associated_graph,
                    handle_consider_location_tag,
                    update_lift_for_moved_anchors,
                    update_lift_door_availability,
                    update_physical_lights,
                    toggle_physical_lights,
                    add_measurement_visuals,
                    update_changed_measurement,
                    update_measurement_for_moved_anchors,
                    handle_model_loaded_events,
                    update_constraint_for_moved_anchors,
                    update_constraint_for_changed_labels,
                    update_changed_constraint,
                    update_model_scenes,
                    update_affiliations,
                    update_members_of_groups.after(update_affiliations),
                    handle_new_sdf_roots,
                    update_model_scales,
                    make_models_selectable,
                    propagate_model_render_layers,
                    handle_new_mesh_primitives,
                    add_drawing_visuals,
                    handle_loaded_drawing,
                    update_drawing_rank,
                    add_physical_camera_visuals,
                    add_wall_visual,
                    handle_update_fuel_cache_requests,
                    read_update_fuel_cache_results,
                    reload_failed_models_with_new_api_key,
                    update_walls_for_moved_anchors,
                    update_walls,
                    update_transforms_for_changed_poses,
                    align_site_drawings,
                    export_lights,
                    ).run_if(in_state(SiteState::Display))
            );
    }
}
