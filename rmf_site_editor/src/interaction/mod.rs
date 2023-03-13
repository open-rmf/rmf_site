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

use crate::site::{update_anchor_transforms, SiteUpdateStage};

pub mod anchor;
pub use anchor::*;

pub mod assets;
pub use assets::*;

pub mod camera_controls;
pub use camera_controls::*;

pub mod cursor;
pub use cursor::*;

pub mod edge;
pub use edge::*;

pub mod gizmo;
pub use gizmo::*;

pub mod lane;
pub use lane::*;

pub mod lift;
pub use lift::*;

pub mod light;
pub use light::*;

pub mod mode;
pub use mode::*;

pub mod outline;
pub use outline::*;

pub mod path;
pub use path::*;

pub mod picking;
pub use picking::*;

pub mod point;
pub use point::*;

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
use bevy_mod_picking::{PickingPlugin, PickingSystem};

#[derive(Default)]
pub struct InteractionPlugin;

#[derive(Debug, Clone, Copy, Hash, PartialEq, Eq)]
pub enum InteractionState {
    Enable,
    Disable,
}

#[derive(Debug, Hash, PartialEq, Eq, Clone, StageLabel)]
pub enum InteractionUpdateStage {
    /// Since parentage can have an effect on visuals, we should wait to add
    /// the visuals until after any orphans have been assigned.
    AddVisuals,
    /// This stage happens after the AddVisuals stage has flushed
    ProcessVisuals,
}

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(InteractionState::Disable)
            .add_stage_after(
                SiteUpdateStage::AssignOrphans,
                InteractionUpdateStage::AddVisuals,
                SystemStage::parallel(),
            )
            .add_stage_after(
                InteractionUpdateStage::AddVisuals,
                InteractionUpdateStage::ProcessVisuals,
                SystemStage::parallel(),
            )
            .add_state_to_stage(
                InteractionUpdateStage::AddVisuals,
                InteractionState::Disable,
            )
            .add_state_to_stage(
                InteractionUpdateStage::ProcessVisuals,
                InteractionState::Disable,
            )
            .add_state_to_stage(CoreStage::PostUpdate, InteractionState::Disable)
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
            .add_event::<ChangePick>()
            .add_event::<Select>()
            .add_event::<Hover>()
            .add_event::<MoveTo>()
            .add_event::<ChangeMode>()
            .add_event::<GizmoClicked>()
            .add_event::<SpawnPreview>()
            .add_plugin(PickingPlugin)
            .add_plugin(OutlinePlugin)
            .add_plugin(CameraControlsPlugin)
            .add_system_set(
                SystemSet::on_update(InteractionState::Enable)
                    .with_system(make_lift_doormat_gizmo)
                    .with_system(update_doormats_for_level_change)
                    .with_system(update_cursor_transform)
                    .with_system(update_picking_cam)
                    .with_system(update_physical_light_visual_cues)
                    .with_system(make_selectable_entities_pickable)
                    .with_system(handle_selection_picking)
                    .with_system(maintain_hovered_entities.after(handle_selection_picking))
                    .with_system(maintain_selected_entities.after(maintain_hovered_entities))
                    .with_system(handle_select_anchor_mode.after(maintain_selected_entities))
                    .with_system(update_anchor_visual_cues.after(maintain_selected_entities))
                    .with_system(update_unassigned_anchor_cues)
                    .with_system(update_anchor_cues_for_mode)
                    .with_system(update_anchor_proximity_xray.after(update_cursor_transform))
                    .with_system(remove_deleted_supports_from_visual_cues)
                    .with_system(remove_orphaned_model_previews)
                    .with_system(make_model_previews_not_selectable)
                    .with_system(update_lane_visual_cues.after(maintain_selected_entities))
                    .with_system(update_edge_visual_cues.after(maintain_selected_entities))
                    .with_system(update_point_visual_cues.after(maintain_selected_entities))
                    .with_system(update_path_visual_cues.after(maintain_selected_entities))
                    .with_system(update_outline_visualization.after(maintain_selected_entities))
                    .with_system(
                        update_cursor_hover_visualization.after(maintain_selected_entities),
                    )
                    .with_system(update_gizmo_click_start.after(maintain_selected_entities))
                    .with_system(update_gizmo_release)
                    .with_system(
                        update_drag_motions
                            .after(update_gizmo_click_start)
                            .after(update_gizmo_release),
                    )
                    .with_system(handle_lift_doormat_clicks.after(update_gizmo_click_start))
                    .with_system(manage_previews)
                    .with_system(update_physical_camera_preview)
                    .with_system(dirty_changed_lifts)
                    .with_system(handle_preview_window_close),
            )
            .add_system_set_to_stage(
                InteractionUpdateStage::AddVisuals,
                SystemSet::on_update(InteractionState::Enable)
                    .with_system(add_anchor_visual_cues)
                    .with_system(remove_interaction_for_subordinate_anchors)
                    .with_system(add_lane_visual_cues)
                    .with_system(add_edge_visual_cues)
                    .with_system(add_point_visual_cues)
                    .with_system(add_path_visual_cues)
                    .with_system(add_outline_visualization)
                    .with_system(add_cursor_hover_visualization)
                    .with_system(add_physical_light_visual_cues),
            )
            .add_system_set_to_stage(
                InteractionUpdateStage::ProcessVisuals,
                SystemSet::on_update(InteractionState::Enable).with_system(propagate_visual_cues),
            )
            .add_system_set(SystemSet::on_exit(InteractionState::Enable).with_system(hide_cursor))
            .add_system_set_to_stage(
                CoreStage::PostUpdate,
                SystemSet::on_update(InteractionState::Enable)
                    .with_system(move_anchor.before(update_anchor_transforms))
                    .with_system(move_pose)
                    .with_system(make_gizmos_pickable),
            )
            .add_system_set_to_stage(
                CoreStage::First,
                SystemSet::new()
                    .with_system(update_picked.after(PickingSystem::UpdateIntersections))
                    .with_system(update_interaction_mode),
            );
    }
}

pub fn set_visibility(entity: Entity, q_visibility: &mut Query<&mut Visibility>, visible: bool) {
    if let Some(mut visibility) = q_visibility.get_mut(entity).ok() {
        visibility.is_visible = visible;
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
