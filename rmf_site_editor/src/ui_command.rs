use std::collections::{HashMap, HashSet};
use bevy::{prelude::*};
use bevy_rapier3d::parry::bounding_volume::BoundingVolume;

use std::collections::hash_map::Iter;

#[derive(Resource, Default)]
pub struct TopLevelMenuExtensions {
    menu_item: HashMap<String, HashMap<String, EventHandle>>,
    empty_hack: HashMap<String, EventHandle>,
    next_handle: u64
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub struct EventHandle {
    handle: u64
}

#[derive(Eq, PartialEq, Debug, Clone)]
pub enum MenuAddError {
    NameIsTakenUp
}

impl TopLevelMenuExtensions {
    pub fn add_item(&mut self, top_item: &String, action: &String) -> Result<EventHandle, MenuAddError> {
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
        let handle = EventHandle{handle: self.next_handle};
        self.next_handle += 1;
        action_map.insert(action.clone(), handle.clone());
        return Ok(handle)
    }

    pub fn iter_with_key(&self, key: &String) -> Iter<'_, String, EventHandle> {
        let Some(item) = self.menu_item.get(key) else {
            return self.empty_hack.iter();
        };

        item.iter()
    }

    pub fn iter_all_without_keys<'a>(&'a self, skip_keys: &'a HashSet<String>) -> impl Iterator<Item = (&String, &HashMap<String, EventHandle>)> + 'a{
        self.menu_item.iter().filter(|(&ref top_level, _)| !skip_keys.contains(top_level)) 
    }
}

pub enum MenuEvent { 
    MenuClickEvent(EventHandle) 
}

impl MenuEvent  {
    pub fn is_same(&self,evt: &EventHandle) -> bool {
        let MenuEvent::MenuClickEvent(sent) = self else {
            return false;
        };
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