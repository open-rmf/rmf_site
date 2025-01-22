use bevy::{ecs::event, prelude::*};
use bevy_impulse::event_streaming_service;
use std::fmt::Debug;

use crate::{EditMenu, MenuEvent, MenuItem};

#[derive(Event, Debug, Clone, Copy)]
pub struct UndoEvent {
    pub action_id: usize,
}

#[derive(Resource)]
struct UndoMenu {
    menu_entity: Entity,
}
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

fn watch_undo_click(
    mut reader: EventReader<MenuEvent>,
    menu_handle: Res<UndoMenu>,
    mut undo_buffer: ResMut<UndoBuffer>,
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

#[derive(Resource, Default)]
pub struct UndoBuffer {
    pub prev_value: usize,
    pub undo_stack: Vec<usize>,
}

impl UndoBuffer {
    pub fn get_next_revision(&mut self) -> usize {
        self.prev_value += 1;
        self.undo_stack.push(self.prev_value);
        self.prev_value
    }

    pub fn undo_last(&mut self) -> Option<usize> {
        self.undo_stack.pop()
    }
}

#[derive(Default)]
pub struct UndoPlugin;

impl Plugin for UndoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<EditMenu>()
            .init_resource::<UndoMenu>()
            .init_resource::<UndoBuffer>()
            .add_event::<UndoEvent>()
            .add_systems(Update, (watch_undo_click));
    }
}
