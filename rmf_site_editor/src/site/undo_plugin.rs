use bevy::{ecs::event, prelude::*};
use bevy_impulse::event_streaming_service;
use std::fmt::Debug;

use crate::{EditMenu, MenuEvent, MenuItem};

/// This represents an undo event. If you want to implement some form of undo-able
/// action, your plugin should watch for this event. Specifically, we should be
#[derive(Event, Debug, Clone, Copy)]
pub struct UndoEvent {
    /// The action id that should be reverted
    pub action_id: usize,
}

/// This struct represents the undo menu item.
#[derive(Resource)]
struct UndoMenu {
    menu_entity: Entity,
}

/// This is responsible for drawing the undo GUI menu
///TODO(arjo): Decouple
impl FromWorld for UndoMenu {
    fn from_world(world: &mut World) -> Self {
        let undo_label = world.spawn(MenuItem::Text("Undo".into())).id();
        let edit_menu = world.resource::<EditMenu>().get();
        world.entity_mut(edit_menu).push_children(&[undo_label]);
        Self {
            menu_entity: undo_label,
        }
    }
}

/// This item watches the GUI menu to detect if an undo action was clicked.
fn watch_undo_click(
    mut reader: EventReader<MenuEvent>,
    menu_handle: Res<UndoMenu>,
    mut undo_buffer: ResMut<RevisionTracker>,
    mut event_writer: EventWriter<UndoEvent>,
) {
    for menu_click in reader.read() {
        if menu_click.clicked() && menu_click.source() == menu_handle.menu_entity {
            let Some(undo_item) = undo_buffer.undo_last() else {
                continue;
            };
            event_writer.send(UndoEvent {
                action_id: undo_item,
            });
        }
    }
}

/// The RevisionTracker resource is used to manage the undo
/// stack. When a plugin wishes to commit an action it should request anew revision
/// from the RevisionTracker. It is up to the plugin to maintain its
/// own history of what state changes ocurred.
#[derive(Resource, Default)]
pub struct RevisionTracker {
    pub(crate) prev_value: usize,
    pub(crate) undo_stack: Vec<usize>,
}

impl RevisionTracker {
    /// Request a new revision id. This is associated with the action you sent
    pub fn get_next_revision(&mut self) -> usize {
        self.prev_value += 1;
        self.undo_stack.push(self.prev_value);
        self.prev_value
    }

    /// Get the last item that was supposed to be undone.
    pub(crate) fn undo_last(&mut self) -> Option<usize> {
        self.undo_stack.pop()
    }
}

#[derive(Default)]
pub struct UndoPlugin;

impl Plugin for UndoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditMenu>()
            .init_resource::<UndoMenu>()
            .init_resource::<RevisionTracker>()
            .add_event::<UndoEvent>()
            .add_systems(Update, (watch_undo_click));
    }
}
