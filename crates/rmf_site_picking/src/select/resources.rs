use bevy_derive::{Deref, DerefMut};
pub use bevy_ecs::prelude::*;
use crossflow::Service;
use bytemuck::TransparentWrapper;

use crate::*;
use web_time::Instant;

/// Used as a resource to keep track of which entity is currently selected.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Resource)]
pub struct Selection(pub Option<Entity>);

/// Used as a resource to keep track of which entity is currently hovered.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Resource)]
pub struct Hovering(pub Option<Entity>);

/// Used to keep track of which entity is currently double clicked.
#[derive(Debug, Clone, Copy)]
pub struct DoubleClickSelection {
    pub last_selected_entity: Option<Entity>,
    pub last_selected_time: Instant,
}

impl Default for DoubleClickSelection {
    fn default() -> Self {
        DoubleClickSelection {
            last_selected_entity: None,
            last_selected_time: Instant::now(),
        }
    }
}

/// A resource to track what kind of blockers are preventing the selection
/// behavior from being active
#[derive(Resource)]
pub struct SelectionBlockers {
    /// An entity is being dragged
    pub dragging: bool,
}

impl SelectionBlockers {
    pub fn blocking(&self) -> bool {
        self.dragging
    }
}

impl Default for SelectionBlockers {
    fn default() -> Self {
        SelectionBlockers { dragging: false }
    }
}

/// Misc Inspector services.
#[derive(Resource)]
pub struct InspectorServiceConfigs {
    /// Workflow that outputs hover and select streams that are compatible with
    /// a general inspector. This service never terminates.
    pub inspector_select_service: Service<(), (), SelectionStreams>,
    pub inspector_cursor_transform: Service<(), ()>,
    pub selection_update: Service<Select, ()>,
}

/// Workflow that updates the [`Selection`] as well as [`Hovered`] and
/// [`Selected`] states in the application.
#[derive(Resource, Debug, TransparentWrapper)]
#[repr(transparent)]
pub struct InspectorService(pub Service<(), ()>);
