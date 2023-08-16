use bevy::prelude::*;
use bevy_rapier3d::parry::bounding_volume::BoundingVolume;
use std::collections::{HashMap, HashSet};

use std::collections::hash_map::Iter;

/// This resource provides a set of tools for adding
#[derive(Resource, Default)]
pub struct TopLevelMenuExtensions {
    /// Map of menu items
    menu_item: HashMap<String, HashMap<String, EventHandle>>,
    /// A hack
    empty_hack: HashMap<String, EventHandle>,
    /// The ID of the next handle
    next_handle: u64,
}

/// Event handles are useful for tracking when
#[derive(Eq, PartialEq, Debug, Clone)]
pub struct EventHandle {
    handle: u64,
}

/// Error describing why we could not add a previous menu item
#[derive(Eq, PartialEq, Debug, Clone)]
pub enum MenuAddError {
    NameIsTakenUp,
}

impl TopLevelMenuExtensions {
    /// Adds a menu item. The `top_item` is the top level menu to place it under
    /// the action is the submenu to be used. If successfully added it returns a
    /// menu handle.
    pub fn add_item(
        &mut self,
        top_item: &String,
        action: &String,
    ) -> Result<EventHandle, MenuAddError> {
        let Some(mut action_map) = self.menu_item.get_mut(top_item) else {
            let mut map = HashMap::new();
            let handle = EventHandle{handle: self.next_handle};
            self.next_handle += 1;
            map.insert(action.clone(), handle.clone());
            self.menu_item.insert(top_item.clone(), map);
            return Ok(handle);
        };

        if let Some(_) = action_map.get(action) {
            return Err(MenuAddError::NameIsTakenUp);
        }
        let handle = EventHandle {
            handle: self.next_handle,
        };
        self.next_handle += 1;
        action_map.insert(action.clone(), handle.clone());
        return Ok(handle);
    }

    /// Ideally these will be private APIs, but for now I did
    /// not want to break the existing code. These are used for drawing the menu.
    pub fn iter_with_key(&self, key: &String) -> Iter<'_, String, EventHandle> {
        let Some(item) = self.menu_item.get(key) else {
            return self.empty_hack.iter();
        };

        item.iter()
    }

    /// Ideally these will be private APIs, but for now I did
    /// not want to break the existing code. These are used for drawing the menu.
    pub fn iter_all_without_keys<'a>(
        &'a self,
        skip_keys: &'a HashSet<String>,
    ) -> impl Iterator<Item = (&String, &HashMap<String, EventHandle>)> + 'a {
        self.menu_item
            .iter()
            .filter(|(&ref top_level, _)| !skip_keys.contains(top_level))
    }
}

/// This event is written when a menu item is clicked.
pub enum MenuEvent {
    MenuClickEvent(EventHandle),
}

impl MenuEvent {
    pub fn is_same(&self, evt: &EventHandle) -> bool {
        let MenuEvent::MenuClickEvent(sent) = self;
        sent == evt
    }
}

pub struct MenuPluginManager;

impl Plugin for MenuPluginManager {
    fn build(&self, app: &mut App) {
        app.add_event::<MenuEvent>()
            .init_resource::<TopLevelMenuExtensions>();
    }
}
