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

pub mod gizmo;
pub use gizmo::*;

pub mod lane;
pub use lane::*;

pub mod lift;
pub use lift::*;

pub mod light;
pub use light::*;

pub mod misc;
pub use misc::*;

pub mod mode;
pub use mode::*;

pub mod picking;
pub use picking::*;

pub mod preview;
pub use preview::*;

pub mod select;
pub use select::*;

pub mod select_anchor;
pub use select_anchor::*;

use bevy::prelude::*;
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
}

/// A unit component to tag entities that are only meant to be visual cues and
/// should be excluded from visualization or analysis of physical objects.
#[derive(Component)]
pub struct VisualCue;

impl Plugin for InteractionPlugin {
    fn build(&self, app: &mut App) {
        app.add_state(InteractionState::Disable)
            .add_stage_after(
                SiteUpdateStage::AssignOrphans,
                InteractionUpdateStage::AddVisuals,
                SystemStage::parallel(),
            )
            .add_state_to_stage(
                InteractionUpdateStage::AddVisuals,
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
                    .with_system(remove_deleted_supports_from_visual_cues)
                    .with_system(update_lane_visual_cues.after(maintain_selected_entities))
                    .with_system(update_misc_visual_cues.after(maintain_selected_entities))
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
                    .with_system(handle_preview_window_close),
            )
            .add_system_set_to_stage(
                InteractionUpdateStage::AddVisuals,
                SystemSet::on_update(InteractionState::Enable)
                    .with_system(add_anchor_visual_cues)
                    .with_system(remove_interaction_for_subordinate_anchors)
                    .with_system(add_lane_visual_cues)
                    .with_system(add_misc_visual_cues)
                    .with_system(add_physical_light_visual_cues),
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
