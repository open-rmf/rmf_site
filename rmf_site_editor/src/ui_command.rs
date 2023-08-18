use bevy::prelude::*;
use std::collections::{HashMap, HashSet};

use std::collections::hash_map::Iter;


#[derive(Component)]
pub struct Menu {
    text: String,
}

impl Menu {
    pub fn from_title(text: String) -> Self {
        Self {text}
    }
    pub fn get(&self) -> String {
        self.text.clone()
    }
}

#[derive(Component)]
pub enum MenuItem {
    Text(String)
}

/// This resource provides the root entity for the file menu
#[derive(Resource)]
pub struct FileMenu {
    /// Map of menu items
    menu_item: Entity
}

impl FileMenu {
    pub fn get(&self) -> Entity {
        return self.menu_item;
    }
}

impl FromWorld for FileMenu {
    fn from_world(world: &mut World) -> Self {
        let menu_item = world.spawn(Menu { text: "File".to_string() }).id();
        Self {
            menu_item
        }
    }
}

pub enum MenuEvent {
    MenuClickEvent(Entity),
}

impl MenuEvent {
    pub fn check_source(&self, evt: &Entity) -> bool {
        let MenuEvent::MenuClickEvent(sent) = self;
        sent == evt
    }
}

pub struct MenuPluginManager;

impl Plugin for MenuPluginManager {
    fn build(&self, app: &mut App) {
        app.add_event::<MenuEvent>()
            .init_resource::<FileMenu>();
    }
}
