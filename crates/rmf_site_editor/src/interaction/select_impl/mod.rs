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
use rmf_site_format::Pending;
use rmf_site_picking::{Hover, Select, Selectable, SelectionFilter};
pub use select_anchor::*;

use bevy::{ecs::system::SystemParam, prelude::*};
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
