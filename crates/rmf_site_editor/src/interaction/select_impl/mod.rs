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
use rmf_site_format::{LiftCabin, Pending};
use rmf_site_picking::{CommonNodeErrors, Hover, Select, Selectable, SelectionFilter};
pub use select_anchor::*;

use anyhow::Error as Anyhow;

use crate::interaction::*;
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

#[derive(Default, Resource)]
pub struct CreationSettings {
    pub direction_alignment: Vec<Vec2>,
}

impl CreationSettings {
    pub fn reset(&mut self) {
        self.direction_alignment = Vec::new();
    }
}

pub fn apply_creation_settings(
    creation_settings: Res<CreationSettings>,
    cursor: Res<Cursor>,
    mut transform: Query<&mut Transform>,
) {
    let Ok(mut frame_tf) = transform.get_mut(cursor.frame) else {
        return;
    };
    for alignment in &creation_settings.direction_alignment {
        frame_tf.translation += alignment.extend(0.0);
    }
}
