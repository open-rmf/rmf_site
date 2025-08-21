use bevy_derive::{Deref, DerefMut};
use bevy_ecs::prelude::*;
use bevy_impulse::{Service, Stream};

use crate::SelectionCandidate;

#[derive(Debug, Clone, Copy, Event)]
pub struct RunSelector {
    /// The select workflow will run this service until it terminates and then
    /// revert back to the inspector selector.
    pub selector: Service<Option<Entity>, ()>,
    /// If there is input for the selector, it will be stored in a [`SelectorInput`]
    /// component in this entity. The entity will be despawned as soon as the
    /// input is extracted.
    pub input: Option<Entity>,
}

/// Used as an event to command a change in the hovered entity.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Event, Stream)]
pub struct Hover(pub Option<Entity>);

/// Used as an event to command a change in the selected entity.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Event, Stream)]
pub struct Select(pub Option<SelectionCandidate>);

impl Select {
    pub fn new(candidate: Option<Entity>) -> Select {
        Select(candidate.map(|c| SelectionCandidate::new(c)))
    }

    pub fn provisional(candidate: Entity) -> Select {
        Select(Some(SelectionCandidate::provisional(candidate)))
    }
}

/// Used as an event to command a change in the double clicked entity.
#[derive(Default, Debug, Clone, Copy, Deref, DerefMut, Event, Stream)]
pub struct DoubleClickSelect(pub Option<Entity>);
