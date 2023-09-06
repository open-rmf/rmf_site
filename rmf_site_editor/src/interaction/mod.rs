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
    update_anchor_transforms, CollisionMeshMarker, ConstraintMarker, DoorMarker, FiducialMarker,
    FloorMarker, LaneMarker, LiftCabin, LiftCabinDoorMarker, LocationTags, MeasurementMarker,
    ModelMarker, SiteUpdateSet, VisualMeshMarker, WallMarker,
};

pub mod anchor;
pub use anchor::*;

pub mod assets;
pub use assets::*;

pub mod camera_controls;
pub use camera_controls::*;

pub mod category_visibility;
pub use category_visibility::*;

pub mod cursor;
pub use cursor::*;

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

pub mod mode;
pub use mode::*;

pub mod model_preview;
pub use model_preview::*;

pub mod outline;
pub use outline::*;

pub mod path;
pub use path::*;

pub mod picking;
pub use picking::*;

pub mod point;
pub use point::*;

pub mod popup;
pub use popup::*;

pub mod preview;
pub use preview::*;

pub mod select;
pub use select::*;

pub mod select_anchor;
pub use select_anchor::*;

pub mod visual_cue;
pub use visual_cue::*;

use bevy::prelude::*;
use bevy_mod_outline::OutlinePlugin;
use bevy_mod_picking::{backend::prelude::PickSet, DefaultPickingPlugins};
use bevy_mod_raycast::update_raycast;
use bevy_polyline::PolylinePlugin;

#[derive(Reflect)]
pub struct SiteRaycastSet;

#[derive(Default)]
pub struct InteractionPlugin;

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
        app.add_state::<InteractionState>()
            .configure_sets(
                Update,
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
                apply_deferred.in_set(InteractionUpdateSet::CommandFlush),
            )
            /*
            .add_stage_after(
                SiteUpdateStage::AssignOrphans,
                InteractionUpdateSet::AddVisuals,
                SystemStage::parallel(),
            )
            .add_stage_after(
                InteractionUpdateSet::AddVisuals,
                InteractionUpdateSet::ProcessVisuals,
                SystemStage::parallel(),
            )
            .add_state_to_stage(
                InteractionUpdateSet::AddVisuals,
                InteractionState::Disable,
            )
            .add_state_to_stage(
                InteractionUpdateSet::ProcessVisuals,
                InteractionState::Disable,
            )
            .add_state_to_stage(CoreStage::PostUpdate, InteractionState::Disable)
            */
            .add_plugin(PolylinePlugin)
            .init_resource::<InteractionAssets>()
            .init_resource::<Cursor>()
            .init_resource::<CameraControls>()
            .init_resource::<Picked>()
            .init_resource::<PickingBlockers>()
            .init_resource::<SelectionBlockers>()
            .init_resource::<Selection>()
            .init_resource::<Hovering>()
            .init_resource::<GizmoState>()
            .init_resource::<InteractionMode>()
            .init_resource::<HiddenSelectAnchorEntities>()
            .add_event::<ChangePick>()
            .add_event::<Select>()
            .add_event::<Hover>()
            .add_event::<MoveTo>()
            .add_event::<ChangeMode>()
            .add_event::<GizmoClicked>()
            .add_event::<SpawnPreview>()
            .add_plugins(DefaultPickingPlugins)
            .add_plugin(OutlinePlugin)
            .add_plugin(CategoryVisibilityPlugin::<DoorMarker>::visible(true))
            .add_plugin(CategoryVisibilityPlugin::<FloorMarker>::visible(true))
            .add_plugin(CategoryVisibilityPlugin::<LaneMarker>::visible(true))
            // TODO(luca) unify the two Lift plugins into a single one?
            .add_plugin(CategoryVisibilityPlugin::<LiftCabin<Entity>>::visible(true))
            .add_plugin(CategoryVisibilityPlugin::<LiftCabinDoorMarker>::visible(
                true,
            ))
            .add_plugin(CategoryVisibilityPlugin::<LocationTags>::visible(true))
            .add_plugin(CategoryVisibilityPlugin::<FiducialMarker>::visible(true))
            .add_plugin(CategoryVisibilityPlugin::<ConstraintMarker>::visible(true))
            .add_plugin(CategoryVisibilityPlugin::<VisualMeshMarker>::visible(true))
            .add_plugin(CategoryVisibilityPlugin::<CollisionMeshMarker>::visible(
                false,
            ))
            .add_plugin(CategoryVisibilityPlugin::<MeasurementMarker>::visible(true))
            .add_plugin(CategoryVisibilityPlugin::<WallMarker>::visible(true))
            .add_plugin(CameraControlsPlugin)
            .add_plugin(ModelPreviewPlugin)
            .add_systems(
                PreUpdate,
                (
                    make_lift_doormat_gizmo,
                    update_doormats_for_level_change,
                    update_cursor_transform,
                    update_picking_cam,
                    update_physical_light_visual_cues,
                    make_selectable_entities_pickable,
                    handle_selection_picking,
                    maintain_hovered_entities.after(handle_selection_picking),
                    maintain_selected_entities.after(maintain_hovered_entities),
                    handle_select_anchor_mode.after(maintain_selected_entities),
                    handle_select_anchor_3d_mode.after(maintain_selected_entities),
                    update_anchor_visual_cues.after(maintain_selected_entities),
                    update_popups.after(maintain_selected_entities),
                    update_unassigned_anchor_cues,
                    update_anchor_cues_for_mode,
                    update_anchor_proximity_xray.after(update_cursor_transform),
                    remove_deleted_supports_from_visual_cues,
                )
                    .run_if(in_state(InteractionState::Enable)),
            )
            // Split the above because of a compile error when the tuple is too large
            .add_systems(
                PreUpdate,
                (
                    make_model_previews_not_selectable,
                    update_lane_visual_cues.after(maintain_selected_entities),
                    update_edge_visual_cues.after(maintain_selected_entities),
                    update_point_visual_cues.after(maintain_selected_entities),
                    update_path_visual_cues.after(maintain_selected_entities),
                    update_outline_visualization.after(maintain_selected_entities),
                    update_highlight_visualization.after(maintain_selected_entities),
                    update_cursor_hover_visualization.after(maintain_selected_entities),
                    update_gizmo_click_start.after(maintain_selected_entities),
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
                Update,
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
                (
                    // TODO(luca) this was placed before update_anchor_transforms
                    move_anchor,
                    move_pose,
                    make_gizmos_pickable,
                )
                    .run_if(in_state(InteractionState::Enable)),
            )
            .add_systems(
                First,
                (
                    update_picked.after(PickSet::PostFocus),
                    update_interaction_mode,
                    // TODO(luca) check in which stage to place this
                    update_raycast::<SiteRaycastSet>,
                ),
            );
    }
}

pub fn set_visibility(entity: Entity, q_visibility: &mut Query<&mut Visibility>, visible: bool) {
    if let Some(mut visibility) = q_visibility.get_mut(entity).ok() {
        *visibility = if visible {
            Visibility::Inherited
        } else {
            Visibility::Hidden
        };
    }
}

fn set_material(
    entity: Entity,
    to_material: &Handle<StandardMaterial>,
    q_materials: &mut Query<&mut Handle<StandardMaterial>>,
) {
    if let Some(mut m) = q_materials.get_mut(entity).ok() {
        *m = to_material.clone();
    }
}
