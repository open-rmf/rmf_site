use std::collections::HashSet;

use bevy_ecs::prelude::*;

#[derive(Component)]
pub struct SelectorInput<T>(pub T);


/// This component is put on entities with meshes to mark them as items that can
/// be interacted with to
#[derive(Component, Clone, Copy, Debug)]
pub struct Selectable {
    /// Toggle whether this entity is selectable
    pub is_selectable: bool,
    /// What element of the site is being selected when this entity is clicked
    pub element: Entity,
}

impl Selectable {
    pub fn new(element: Entity) -> Self {
        Selectable {
            is_selectable: true,
            element,
        }
    }
}

#[derive(Component, Debug, PartialEq, Eq)]
pub struct Selected {
    /// This object has been selected
    pub is_selected: bool,
    /// Another object is selected but wants this entity to be highlighted
    pub support_selected: HashSet<Entity>,
}

impl Selected {
    pub fn cue(&self) -> bool {
        self.is_selected || !self.support_selected.is_empty()
    }
}

impl Default for Selected {
    fn default() -> Self {
        Self {
            is_selected: false,
            support_selected: Default::default(),
        }
    }
}

/// Component to track whether an element should be viewed in the Hovered state
/// for the selection tool.
#[derive(Component, Debug, PartialEq, Eq)]
pub struct Hovered {
    /// The cursor is hovering on this object specifically
    pub is_hovered: bool,
    /// The cursor is hovering on a different object which wants this entity
    /// to be highlighted.
    pub support_hovering: HashSet<Entity>,
}

impl Hovered {
    pub fn cue(&self) -> bool {
        self.is_hovered || !self.support_hovering.is_empty()
    }
}

impl Default for Hovered {
    fn default() -> Self {
        Self {
            is_hovered: false,
            support_hovering: Default::default(),
        }
    }
}

/// Frame for cursor
#[derive(Component)]
pub struct CursorFrame;

/// A unit component that indicates the entity is only for previewing and
/// should never be interacted with. This is applied to the "anchor" that is
/// attached to the cursor.
#[derive(Component, Clone, Copy, Debug)]
pub struct Preview;