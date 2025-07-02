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

use crate::site::{
    update_anchor_transforms, CollisionMeshMarker, CurrentEditDrawing, CurrentLevel, DoorMarker,
    FiducialMarker, FloorMarker, LaneMarker, LiftCabin, LiftCabinDoorMarker, LocationTags,
    MeasurementMarker, SiteUpdateSet, ToggleLiftDoorAvailability, VisualMeshMarker, WallMarker,
};

pub mod anchor;
pub use anchor::*;

pub mod assets;
pub use assets::*;

use rmf_site_camera::plugins::{BlockerRegistryPlugin, CameraSetupPlugin};
pub use rmf_site_camera::*;

pub mod category_visibility;
pub use category_visibility::*;

pub mod edge;
pub use edge::*;

pub mod gizmo;
pub use gizmo::*;

pub mod highlight;
pub use highlight::*;

pub mod lane;
pub use lane::*;

pub mod lift;
pub use lift::*;

pub mod light;
pub use light::*;

pub mod model;
pub use model::*;

pub mod model_preview;
pub use model_preview::*;

pub mod outline;
pub use outline::*;

pub mod path;
pub use path::*;

pub mod cursor;
pub use cursor::*;

// pub mod picking;
// pub use picking::*;



use rmf_site_picking::*;

pub mod point;
pub use point::*;

pub mod popup;
pub use popup::*;

pub mod preview;
pub use preview::*;

pub mod select_impl;
pub use select_impl::*;

// pub mod select;
// pub use select::*;

// pub mod visual_cue;
// pub use visual_cue::*;

use bevy::prelude::*;
use bevy_mod_outline::OutlinePlugin;

#[derive(Default)]
pub struct InteractionPlugin {
    headless: bool,
}

impl InteractionPlugin {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn headless(mut self, is_headless: bool) -> Self {
        self.headless = is_headless;
        self
    }
}

#[derive(Debug, Clone, Copy, Default, Hash, PartialEq, Eq, States)]
pub enum InteractionState {
    Enable,
    #[default]
    Disable,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, SystemSet)]
pub enum InteractionUpdateSet {
    /// Since parentage can have an effect on visuals, we should wait to add
    /// the visuals until after any orphans have been assigned.
    AddVisuals,
    /// Force a command flush between the two sets
    CommandFlush,
    /// This set happens after the AddVisuals set has flushed
    ProcessVisuals,
    // TODO(luca) should we have a command flush after process visuals?
}

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.init_state::<InteractionState>()
            .init_resource::<GizmoBlockers>()
            .configure_sets(
                PostUpdate,
                (
                    SiteUpdateSet::AssignOrphansFlush,
                    InteractionUpdateSet::AddVisuals,
                    InteractionUpdateSet::CommandFlush,
                    InteractionUpdateSet::ProcessVisuals,
                )
                    .chain(),
            )
            .add_systems(
                Update,
                ApplyDeferred.in_set(InteractionUpdateSet::CommandFlush),
            )
            .add_plugins(MeshPickingPlugin)
            .init_resource::<InteractionAssets>()
            .init_resource::<Cursor>()
            .init_resource::<PickingBlockers>()
            .init_resource::<UiHovered>()
            .init_resource::<IteractionMaskHovered>()
            .add_plugins(CameraControlsBlocker::<UiHovered>::default())
            .add_plugins(BlockerRegistryPlugin::<PickingBlockers>::default())
            .add_plugins(PickBlockerRegistration::<UiHovered>::default())
            .add_plugins(PickBlockerRegistration::<IteractionMaskHovered>::default())
            .init_resource::<Picked>()
            .init_resource::<GizmoState>()
            .init_resource::<CurrentEditDrawing>()
            .init_resource::<CurrentLevel>()
            .insert_resource(HighlightAnchors(false))
            .add_event::<ChangePick>()
            .add_event::<MoveTo>()
            .add_event::<GizmoClicked>()
            .add_event::<SpawnPreview>()
            .add_event::<ToggleLiftDoorAvailability>()
            .add_plugins((
                OutlinePlugin,
                CategoryVisibilityPlugin::<DoorMarker>::visible(true),
                CategoryVisibilityPlugin::<FloorMarker>::visible(true),
                CategoryVisibilityPlugin::<LaneMarker>::visible(true),
                CategoryVisibilityPlugin::<LiftCabin<Entity>>::visible(true),
                CategoryVisibilityPlugin::<LiftCabinDoorMarker>::visible(true),
                CategoryVisibilityPlugin::<LocationTags>::visible(true),
                CategoryVisibilityPlugin::<FiducialMarker>::visible(true),
                CategoryVisibilityPlugin::<VisualMeshMarker>::visible(true),
                CategoryVisibilityPlugin::<CollisionMeshMarker>::visible(false),
                CategoryVisibilityPlugin::<MeasurementMarker>::visible(true),
                CategoryVisibilityPlugin::<WallMarker>::visible(true),
            ))
            .add_plugins((
                CameraSetupPlugin,
                ModelPreviewPlugin,
                InspectorServicePlugin::default(),
                AnchorSelectionPlugin::default(),
                ObjectPlacementPlugin::default(),
                SelectionPlugin::<InspectorService>::default(),
            ));

        if !self.headless {
            app.add_systems(
                Update,
                (
                    make_lift_doormat_gizmo,
                    update_doormats_for_level_change,
                    update_physical_light_visual_cues,
                    make_selectable_entities_pickable,
                    update_anchor_visual_cues.after(SelectionServiceStages::Select),
                    update_popups.after(SelectionServiceStages::Select),
                    update_unassigned_anchor_cues,
                    update_anchor_proximity_xray.after(SelectionServiceStages::PickFlush),
                    remove_deleted_supports_from_visual_cues,
                    on_highlight_anchors_change,
                )
                    .run_if(in_state(InteractionState::Enable)),
            )
            // Split the above because of a compile error when the tuple is too large
            .add_systems(
                Update,
                (
                    update_model_instance_visual_cues.after(SelectionServiceStages::Select),
                    update_lane_visual_cues.after(SelectionServiceStages::Select),
                    update_edge_visual_cues.after(SelectionServiceStages::Select),
                    update_point_visual_cues.after(SelectionServiceStages::Select),
                    update_path_visual_cues.after(SelectionServiceStages::Select),
                    update_outline_visualization.after(SelectionServiceStages::Select),
                    update_highlight_visualization.after(SelectionServiceStages::Select),
                    update_cursor_hover_visualization.after(SelectionServiceStages::Select),
                    update_gizmo_click_start.after(SelectionServiceStages::Select),
                    update_gizmo_release,
                    update_drag_motions
                        .after(update_gizmo_click_start)
                        .after(update_gizmo_release),
                    handle_lift_doormat_clicks.after(update_gizmo_click_start),
                    manage_previews,
                    update_physical_camera_preview,
                    dirty_changed_lifts,
                    handle_preview_window_close,
                )
                    .run_if(in_state(InteractionState::Enable)),
            )
            .add_systems(
                PostUpdate,
                (
                    add_anchor_visual_cues,
                    remove_interaction_for_subordinate_anchors,
                    add_lane_visual_cues,
                    add_edge_visual_cues,
                    add_point_visual_cues,
                    add_path_visual_cues,
                    add_outline_visualization,
                    add_highlight_visualization,
                    add_cursor_hover_visualization,
                    add_cursor_hover_visualization,
                    add_physical_light_visual_cues,
                    add_popups,
                )
                    .run_if(in_state(InteractionState::Enable))
                    .in_set(InteractionUpdateSet::AddVisuals),
            )
            .add_systems(
                Update,
                propagate_visual_cues
                    .run_if(in_state(InteractionState::Enable))
                    .in_set(InteractionUpdateSet::ProcessVisuals),
            )
            .add_systems(OnExit(InteractionState::Enable), hide_cursor)
            .add_systems(
                PostUpdate,
                (move_anchor.before(update_anchor_transforms), move_pose)
                    .run_if(in_state(InteractionState::Enable)),
            )
            .add_systems(First, update_picked);
        }
    }
}

// pub fn set_visibility(entity: Entity, q_visibility: &mut Query<&mut Visibility>, visible: bool) {
//     if let Some(mut visibility) = q_visibility.get_mut(entity).ok() {
//         let v = if visible {
//             Visibility::Inherited
//         } else {
//             Visibility::Hidden
//         };

//         // Avoid a mutable access if nothing actually needs to change
//         if *visibility != v {
//             *visibility = v;
//         }
//     }
// }

fn set_material(
    entity: Entity,
    to_material: &Handle<StandardMaterial>,
    q_materials: &mut Query<&mut MeshMaterial3d<StandardMaterial>>,
) {
    if let Some(mut m) = q_materials.get_mut(entity).ok() {
        *m = MeshMaterial3d(to_material.clone());
    }
}
