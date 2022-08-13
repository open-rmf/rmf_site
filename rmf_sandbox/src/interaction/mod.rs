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

pub mod camera_controls;
pub use camera_controls::*;

pub mod cursor;
pub use cursor::*;

pub mod drag;
pub use drag::*;

pub mod lane;
pub use lane::*;

pub mod picking;
pub use picking::*;

pub mod select;
pub use select::*;

use bevy::prelude::*;
use bevy_mod_picking::PickingSystem;

#[derive(Default)]
pub struct InteractionPlugin<T> {
    for_app_state: T,
}

impl<T> InteractionPlugin<T> {
    pub fn new(for_app_state: T) -> Self {
        Self { for_app_state }
    }
}

impl<T: Send + Sync + Clone + Hash + Eq + Debug + 'static> Plugin for InteractionPlugin<T> {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<InteractionAssets>()
            .init_resource::<Dragging>()
            .add_event::<ElementDeleted>()
            .add_startup_system(init_cursor)
            .add_plugin(CameraControlsPlugin)
            .add_system_set(
                SystemSet::on_update(self.for_app_state.clone())
                    .with_system(update_cursor_transform.after(PickingSystem::UpdateIntersections))
                    .with_system(make_selectable_entities_pickable)
                    .with_system(update_anchor_visual_cues)
                    .with_system(update_lane_visual_cues)
                    .with_system(update_floor_and_wall_visual_cues)
                    .with_system(remove_deleted_supports_from_interactions)
                    .with_system(make_gizmos_pickable)
                    .with_system(update_drag_click_start)
                    .with_system(update_drag_release)
                    .with_system(
                        update_drag_motions
                        .after(update_drag_click_start)
                        .after(update_drag_release)
                    ),
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

// TODO(MXG): Customize the behavior of floor, wall, and model visual cues
#[derive(Component)]
pub struct FloorVisualCue;

#[derive(Component)]
pub struct WallVisualCue;

#[derive(Component)]
pub struct DefaultVisualCue;

pub fn update_floor_and_wall_visual_cues(
    floors: Query<&Hovering, With<FloorVisualCue>>,
    walls: Query<&Hovering, With<WallVisualCue>>,
    everything_else: Query<&Hovering, With<DefaultVisualCue>>,
    cursor: Query<Entity, With<Cursor>>,
    mut visibility: Query<&mut Visibility>,
) {
    for hovering in floors
        .iter()
        .chain(walls.iter())
        .chain(everything_else.iter())
    {
        if hovering.cue() {
            if let Some(mut v) = visibility.get_mut(cursor.single()).ok() {
                v.is_visible = true;
            }
        }
    }
}
