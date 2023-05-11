use bevy::{prelude::*};
use rmf_site_format::{Anchor, Pose};
use std::collections::VecDeque;

use crate::site::Subordinate;

#[derive(Debug, Clone, Copy, PartialEq)]
pub enum UndoEvent {
    MoveAnchor(Entity, Transform),
    MovePoseObject(Entity, Transform),
    OtherEvent
}

#[derive(Resource, Default)]
pub struct UndoStack {
    pub actions: VecDeque<UndoEvent>
}

#[derive(Debug, Clone, Copy, Default, PartialEq)]
pub struct TriggerUndo {}

pub fn perform_undo(
    mut ev_trigger_undo: EventReader<TriggerUndo>,
    mut undo_stack: ResMut<UndoStack>,
    mut anchors: Query<&mut Anchor, Without<Subordinate>>,
    mut poses: Query<&mut Pose>,
)
{
    for _undo in ev_trigger_undo.iter() {
        if let Some(front) =  undo_stack.actions.pop_front() {
            match front {
                UndoEvent::MoveAnchor(entity, old_pose) => {
                    if let Ok(mut anchor) = anchors.get_mut(entity) {
                        anchor.move_to(&old_pose);
                    }
                },
                UndoEvent::MovePoseObject(entity, old_pose) => {
                    if let Ok(mut pose) = poses.get_mut(entity) {
                        pose.align_with(&old_pose);
                    }
                },
                _ => {}
            }
        }
    }
}