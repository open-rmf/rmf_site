/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

pub mod create_edges;
use create_edges::*;

pub mod create_path;
use create_path::*;

pub mod create_point;
use create_point::*;

pub mod place_object;
pub use place_object::*;

pub mod place_object_2d;
pub use place_object_2d::*;

pub mod replace_point;
use replace_point::*;

pub mod replace_side;
use replace_side::*;

pub mod select_anchor;
pub use select_anchor::*;

pub mod selection_alignment;
pub use selection_alignment::*;

use rmf_site_format::{LiftCabin, Pending};
use rmf_site_picking::{CommonNodeErrors, Hover, Select, Selectable, SelectionFilter};

use anyhow::Error as Anyhow;

use bevy::{
    ecs::{relationship::AncestorIter, system::SystemParam},
    prelude::*,
};
use rmf_site_picking::Preview;

pub const SELECT_ANCHOR_MODE_LABEL: &'static str = "select_anchor";

#[derive(SystemParam)]
pub struct InspectorFilter<'w, 's> {
    selectables: Query<'w, 's, &'static Selectable, (Without<Preview>, Without<Pending>)>,
}

impl<'w, 's> SelectionFilter for InspectorFilter<'w, 's> {
    fn filter_pick(&mut self, select: Entity) -> Option<Entity> {
        self.selectables
            .get(select)
            .ok()
            .map(|selectable| selectable.element)
    }
    fn filter_select(&mut self, target: Entity) -> Option<Entity> {
        Some(target)
    }
    fn on_click(&mut self, hovered: Hover) -> Option<Select> {
        Some(Select::new(hovered.0))
    }
}

/// How should a selection tool behave if the level is changed while the tool is active?
#[derive(Debug, Default, Clone, Copy)]
pub enum LevelChangeContinuity {
    /// When the level is changed during a creation workflow, behave as though
    /// the user has asked to begin creating a separate object, no matter what
    /// state the object was in previously. This should be used for objects that
    /// must be fully contained to a single level.
    #[default]
    Separate,
    /// When the level is changed during a creation workflow, continue creating
    /// whatever object was in progress. This should be used for objects that
    /// are allowed to be connected across multiple levels.
    Continuous,
}

/// Check if these anchors co-exist in the same level. This will account for
/// cases where you are connecting to site or lift anchors.
pub fn are_anchors_siblings(
    a: Entity,
    b: Entity,
    parents: &Query<&ChildOf>,
    lifts: &Query<(), With<LiftCabin<Entity>>>,
) -> Result<bool, Option<Anyhow>> {
    let parent_of_a = parents.get(a).or_broken_query()?.parent();
    let parent_of_b = parents.get(b).or_broken_query()?.parent();
    let mut are_siblings =
        AncestorIter::new(&parents, b).any(|e| e == parent_of_a || lifts.contains(e));
    if !are_siblings {
        are_siblings =
            AncestorIter::new(&parents, a).any(|e| e == parent_of_b || lifts.contains(e));
    }

    Ok(are_siblings)
}

/// Settings that affect the behavior of how objects get created.
#[derive(Resource)]
pub struct CreationSettings {
    /// Should we be trying to align
    pub alignment_on: bool,
    /// Objects further than this from the cursor will not be considered when
    /// calculating alignment.
    pub alignment_window: Option<f32>,
    /// When calculating alignment, the cursor will be able to move within this
    /// radius (in-world meters) without triggering an update of the cache.
    pub alignment_cache_deadzone: f32,
}

impl Default for CreationSettings {
    fn default() -> Self {
        Self {
            alignment_on: false,
            alignment_window: Some(5.0),
            alignment_cache_deadzone: 1.0,
        }
    }
}

pub fn update_selection_settings_for_keyboard(
    keyboard_input: Res<ButtonInput<KeyCode>>,
    mut creation_settings: ResMut<CreationSettings>,
) {
    let alignment_on = keyboard_input.pressed(KeyCode::ShiftLeft);

    // Check the value before mutating the resource so we get cleaner change tracking
    if creation_settings.alignment_on != alignment_on {
        creation_settings.alignment_on = alignment_on;
    }
}
