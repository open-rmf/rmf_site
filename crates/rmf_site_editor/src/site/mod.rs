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

pub mod drawing_editor;
pub use drawing_editor::{alignment, *};

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

pub mod headless_export;
pub use headless_export::*;

pub mod inclusion;
pub use inclusion::*;

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

pub mod modifier;
pub use modifier::*;

pub mod nav_graph;
pub use nav_graph::*;

pub mod path;
pub use path::*;

pub mod physical_camera;
pub use physical_camera::*;

pub mod pose;
pub use pose::*;

pub mod property_plugin;
pub use property_plugin::*;

pub mod primitive_shape;
pub use primitive_shape::*;

pub mod recall_plugin;
pub use recall_plugin::RecallPlugin;

pub mod robot;
#[allow(unused_imports)]
pub use robot::*;

pub mod robot_properties;
pub use robot_properties::*;

pub mod save;
pub use save::*;

pub mod scenario;
pub use scenario::*;

pub mod sdf_exporter;
pub use sdf_exporter::*;

pub mod site;
pub use site::*;

pub mod site_visualizer;
pub use site_visualizer::*;

pub mod task;
pub use task::*;

pub mod slotcar;
pub use slotcar::*;

pub mod texture;
pub use texture::*;

pub mod util;
pub use util::*;

pub mod view_menu;
pub use view_menu::*;

pub mod wall;
pub use wall::*;

use crate::recency::{RecencyRank, RecencyRankingPlugin};
use crate::{AppState, RegisterIssueType};
pub use rmf_site_format::{DirectionalLight, PointLight, SpotLight, Style, *};

use bevy::{prelude::*, render::view::visibility::VisibilitySystems, transform::TransformSystem};

use bevy_infinite_grid::*;

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
    /// Placed between transform and visibility propagation, to avoid one frame delays
    BetweenTransformAndVisibility,
    /// Flush the set above
    BetweenTransformAndVisibilityFlush,
    /// Used to force a command flush after the change plugin's process changes
    ProcessChanges,
    /// Flush the set above
    ProcessChangesFlush,
}

pub struct SitePlugin;

impl Plugin for SitePlugin {
    fn build(&self, app: &mut App) {
        add_site_icons(app);
        app.configure_sets(
            PreUpdate,
            (
                SiteUpdateSet::ProcessChanges,
                SiteUpdateSet::ProcessChangesFlush,
            )
                .chain(),
        )
        .add_systems(
            PreUpdate,
            ApplyDeferred.in_set(SiteUpdateSet::ProcessChangesFlush),
        )
        .configure_sets(
            PostUpdate,
            (
                SiteUpdateSet::AssignOrphans,
                SiteUpdateSet::AssignOrphansFlush,
                TransformSystem::TransformPropagate,
                SiteUpdateSet::BetweenTransformAndVisibility,
                SiteUpdateSet::BetweenTransformAndVisibilityFlush,
                VisibilitySystems::VisibilityPropagate,
                // TODO(luca) remove this when https://github.com/bevyengine/bevy/pull/19064 (or
                // alternative fix) is merged and released
                bevy::asset::AssetEvents,
            )
                .chain(),
        )
        .add_systems(
            PostUpdate,
            ApplyDeferred.in_set(SiteUpdateSet::BetweenTransformAndVisibilityFlush),
        )
        .add_systems(
            PostUpdate,
            ApplyDeferred.in_set(SiteUpdateSet::AssignOrphansFlush),
        )
        .insert_resource(ClearColor(Color::srgb(0., 0., 0.)))
        .init_resource::<SiteAssets>()
        .init_resource::<CurrentLevel>()
        .init_resource::<CurrentScenario>()
        .init_resource::<DefaultScenario>()
        .init_resource::<ExportHandlers>()
        .init_resource::<Trashcan>()
        .init_resource::<PhysicalLightToggle>()
        .add_event::<LoadSite>()
        .add_event::<ImportNavGraphs>()
        .add_event::<ChangeCurrentSite>()
        .add_event::<ChangeCurrentScenario>()
        .add_event::<ChangeDefaultScenario>()
        .add_event::<CreateScenario>()
        .add_event::<RemoveScenario>()
        .add_event::<SaveSite>()
        .add_event::<ExportLights>()
        .add_event::<ConsiderAssociatedGraph>()
        .add_event::<ConsiderLocationTag>()
        .add_event::<MergeGroups>()
        .add_plugins((
            ChangePlugin::<AssociatedGraphs<Entity>>::default(),
            RecallPlugin::<RecallAssociatedGraphs<Entity>>::default(),
            ChangePlugin::<Motion>::default(),
            RecallPlugin::<RecallMotion>::default(),
            ChangePlugin::<ReverseLane>::default(),
            RecallPlugin::<RecallReverseLane>::default(),
            ChangePlugin::<NameOfSite>::default(),
            ChangePlugin::<NameInSite>::default(),
            ChangePlugin::<Pose>::default(),
            ChangePlugin::<Scale>::default(),
            ChangePlugin::<Distance>::default(),
            ChangePlugin::<Texture>::default(),
        ))
        .add_plugins((
            ChangePlugin::<DoorType>::default(),
            RecallPlugin::<RecallDoorType>::default(),
            ChangePlugin::<LevelElevation>::default(),
            ChangePlugin::<LiftCabin<Entity>>::default(),
            RecallPlugin::<RecallLiftCabin<Entity>>::default(),
            ChangePlugin::<AssetSource>::default(),
            RecallPlugin::<RecallAssetSource>::default(),
            ChangePlugin::<PrimitiveShape>::default(),
            RecallPlugin::<RecallPrimitiveShape>::default(),
            ChangePlugin::<PixelsPerMeter>::default(),
            ChangePlugin::<PhysicalCameraProperties>::default(),
            ChangePlugin::<LightKind>::default(),
            RecallPlugin::<RecallLightKind>::default(),
            ChangePlugin::<DisplayColor>::default(),
            ChangePlugin::<LocationTags>::default(),
        ))
        .add_plugins((
            RecallPlugin::<RecallLocationTags>::default(),
            ChangePlugin::<Visibility>::default(),
            ChangePlugin::<LayerVisibility>::default(),
            ChangePlugin::<GlobalFloorVisibility>::default(),
            ChangePlugin::<GlobalDrawingVisibility>::default(),
            ChangePlugin::<PreferredSemiTransparency>::default(),
            ChangePlugin::<Affiliation<Entity>>::default(),
            RecencyRankingPlugin::<NavGraphMarker>::default(),
            RecencyRankingPlugin::<FloorMarker>::default(),
            RecencyRankingPlugin::<DrawingMarker>::default(),
            DeletionPlugin,
            DrawingEditorPlugin,
            SiteVisualizerPlugin,
            ModelLoadingPlugin::default(),
            FuelPlugin::default(),
        ))
        .add_plugins((
            ChangePlugin::<ModelProperty<AssetSource>>::default(),
            ChangePlugin::<ModelProperty<Scale>>::default(),
            ChangePlugin::<ModelProperty<IsStatic>>::default(),
            ChangePlugin::<ModelProperty<Robot>>::default(),
            ChangePlugin::<Task>::default(),
            PropertyPlugin::<Pose, InstanceMarker>::default(),
            PropertyPlugin::<Inclusion, InstanceMarker>::default(),
            PropertyPlugin::<Inclusion, Task>::default(),
            PropertyPlugin::<TaskParams, Task>::default(),
            PropertyPlugin::<OnLevel<Entity>, Robot>::default(),
            SlotcarSdfPlugin,
        ))
        .add_plugins((InfiniteGridPlugin,))
        .add_issue_type(&DUPLICATED_DOOR_NAME_ISSUE_UUID, "Duplicate door name")
        .add_issue_type(&DUPLICATED_LIFT_NAME_ISSUE_UUID, "Duplicate lift name")
        .add_issue_type(
            &FIDUCIAL_WITHOUT_AFFILIATION_ISSUE_UUID,
            "Fiducial without affiliation",
        )
        .add_issue_type(&DUPLICATED_DOCK_NAME_ISSUE_UUID, "Duplicated dock name")
        .add_issue_type(&UNCONNECTED_ANCHORS_ISSUE_UUID, "Unconnected anchors")
        .add_systems(Update, (load_site, import_nav_graph))
        .add_systems(
            PreUpdate,
            (
                update_lift_cabin,
                update_lift_edge,
                update_drawing_pixels_per_meter,
                update_drawing_children_to_pixel_coordinates,
                check_for_duplicated_door_names,
                check_for_duplicated_lift_names,
                check_for_duplicated_dock_names,
                check_for_fiducials_without_affiliation,
                check_for_close_unconnected_anchors,
                check_for_orphan_model_instances,
                check_for_hidden_model_instances,
                check_for_accidentally_moved_instances,
                check_for_invalid_level_assignments,
                fetch_image_for_texture,
                detect_last_selected_texture::<FloorMarker>,
                apply_last_selected_texture::<FloorMarker>
                    .after(detect_last_selected_texture::<FloorMarker>),
                detect_last_selected_texture::<WallMarker>,
                apply_last_selected_texture::<WallMarker>
                    .after(detect_last_selected_texture::<WallMarker>),
                update_material_for_display_color,
            )
                .after(SiteUpdateSet::ProcessChangesFlush)
                .run_if(AppState::in_displaying_mode()),
        )
        .add_systems(
            Update,
            (save_site, change_site.after(load_site)).run_if(AppState::in_displaying_mode()),
        )
        .add_systems(
            PostUpdate,
            (
                assign_orphan_anchors_to_parent,
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
                add_physical_lights,
                clear_trashcan,
            )
                .run_if(AppState::in_displaying_mode())
                .in_set(SiteUpdateSet::AssignOrphans),
        )
        .add_systems(
            PostUpdate,
            (
                add_wall_visual,
                update_walls_for_moved_anchors,
                update_walls,
                update_transforms_for_changed_poses,
                align_site_drawings,
                export_lights,
                set_camera_transform_for_changed_site,
            )
                .run_if(AppState::in_displaying_mode())
                .in_set(SiteUpdateSet::BetweenTransformAndVisibility),
        )
        .add_systems(
            PostUpdate,
            (
                update_anchor_transforms,
                add_door_visuals,
                update_changed_door,
                update_door_for_moved_anchors,
                add_floor_visuals,
                update_floors,
                update_floors_for_moved_anchors,
                update_floors_for_changed_lifts,
                update_floor_visibility,
                update_drawing_visibility,
                add_lane_visuals,
                add_location_visuals,
                add_fiducial_visuals,
                update_level_visibility,
                handle_remove_scenarios.before(update_current_scenario),
                update_current_scenario.before(handle_create_scenarios),
                handle_create_scenarios,
            )
                .run_if(AppState::in_displaying_mode())
                .in_set(SiteUpdateSet::BetweenTransformAndVisibility),
        )
        .add_systems(
            PostUpdate,
            (
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
            )
                .run_if(AppState::in_displaying_mode())
                .in_set(SiteUpdateSet::BetweenTransformAndVisibility),
        )
        .add_systems(
            PostUpdate,
            (
                add_measurement_visuals,
                update_changed_measurement,
                update_measurement_for_moved_anchors,
                update_model_instances::<AssetSource>,
                update_model_instances::<Scale>,
                update_model_instances::<IsStatic>,
                update_affiliations,
                update_members_of_groups.after(update_affiliations),
                update_model_scales,
                handle_new_primitive_shapes,
                add_drawing_visuals,
                handle_loaded_drawing,
                update_drawing_rank,
                add_physical_camera_visuals,
                check_selected_is_included,
                check_for_missing_root_modifiers::<InstanceMarker>,
                update_default_scenario,
            )
                .run_if(AppState::in_displaying_mode())
                .in_set(SiteUpdateSet::BetweenTransformAndVisibility),
        )
        .add_observer(handle_inclusion_change_for_model_visibility)
        .add_observer(handle_on_level_change);
    }
}
