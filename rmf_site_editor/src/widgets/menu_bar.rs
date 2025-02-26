/*
 * Copyright (C) 2023 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

use crate::widgets::prelude::*;
use crate::widgets::{HeaderPanelPlugin, HeaderTilePlugin, StandardSiteObjectCreationPlugin};

use bevy::ecs::query::Has;
use bevy::prelude::*;
use bevy_egui::egui::{self, Button, Ui};

/// Add the standard menu bar to the application.
#[derive(Default)]
pub struct MenuBarPlugin {}

impl Plugin for MenuBarPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            HeaderPanelPlugin::default(),
            MenuDropdownPlugin::default(),
            StandardSiteObjectCreationPlugin::default(),
        ))
        .add_event::<MenuEvent>()
        .init_resource::<FileMenu>()
        .init_resource::<ToolMenu>()
        .init_resource::<ViewMenu>();
    }
}

/// Add a widget housing all the menu dropdown options
#[derive(Default)]
pub struct MenuDropdownPlugin {}

impl Plugin for MenuDropdownPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(HeaderTilePlugin::<MenuDropdowns>::new());
    }
}

#[derive(SystemParam)]
struct MenuDropdowns<'w, 's> {
    menus: Query<'w, 's, (&'static Menu, Entity)>,
    menu_items: Query<'w, 's, (&'static mut MenuItem, Has<MenuDisabled>)>,
    extension_events: EventWriter<'w, MenuEvent>,
    view_menu: Res<'w, ViewMenu>,
    file_menu: Res<'w, FileMenu>,
    children: Query<'w, 's, &'static Children>,
    top_level_components: Query<'w, 's, ((), Without<Parent>)>,
}

impl<'w, 's> WidgetSystem<Tile> for MenuDropdowns<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        ui.menu_button("File", |ui| {
            render_sub_menu(
                ui,
                &params.file_menu.get(),
                &params.children,
                &params.menus,
                &params.menu_items,
                &mut params.extension_events,
                true,
            );
        });
        ui.menu_button("View", |ui| {
            render_sub_menu(
                ui,
                &params.view_menu.get(),
                &params.children,
                &params.menus,
                &params.menu_items,
                &mut params.extension_events,
                true,
            );
        });

        for (_, entity) in params.menus.iter().filter(|(_, entity)| {
            params.top_level_components.contains(*entity)
                && (*entity != params.file_menu.get() && *entity != params.view_menu.get())
        }) {
            render_sub_menu(
                ui,
                &entity,
                &params.children,
                &params.menus,
                &params.menu_items,
                &mut params.extension_events,
                false,
            );
        }
        ui.separator();
    }
}

/// Adding this to an entity to an entity with the [`MenuItem`] component
/// will grey out and disable a [`MenuItem`].
#[derive(Component)]
pub struct MenuDisabled;

/// This component represents a menu. Menus and menu items
/// can be arranged in trees using bevy's own parent-child system.
#[derive(Component)]
pub struct Menu {
    text: String,
}

impl Menu {
    /// Create a new menu from the title
    pub fn from_title(text: String) -> Self {
        Self { text }
    }

    /// Retrieve the menu name
    fn get(&self) -> String {
        self.text.clone()
    }
}

/// Create a new menu item
#[derive(Component)]
#[non_exhaustive]
pub enum MenuItem {
    /// Text + Shortcut hint if available
    Text(TextMenuItem),
    CheckBox(String, bool),
}

impl MenuItem {
    pub fn checkbox_value(&self) -> Option<bool> {
        match self {
            MenuItem::CheckBox(_, value) => Some(*value),
            _ => None,
        }
    }

    pub fn checkbox_value_mut(&mut self) -> Option<&mut bool> {
        match self {
            MenuItem::CheckBox(_, ref mut value) => Some(value),
            _ => None,
        }
    }
}

pub struct TextMenuItem {
    pub text: String,
    pub shortcut: Option<String>,
}

impl From<&str> for TextMenuItem {
    fn from(text: &str) -> Self {
        Self {
            text: text.into(),
            shortcut: None,
        }
    }
}

impl TextMenuItem {
    pub fn new(text: &str) -> Self {
        text.into()
    }

    pub fn shortcut(mut self, shortcut: &str) -> Self {
        self.shortcut = Some(shortcut.into());
        self
    }
}

/// This resource provides the root entity for the file menu
#[derive(Resource)]
pub struct FileMenu {
    /// Map of menu items
    menu_item: Entity,
}

impl FileMenu {
    pub fn get(&self) -> Entity {
        return self.menu_item;
    }
}

impl FromWorld for FileMenu {
    fn from_world(world: &mut World) -> Self {
        let menu_item = world
            .spawn(Menu {
                text: "File".to_string(),
            })
            .id();
        Self { menu_item }
    }
}

/// This resource provides the root entity for the tool menu
#[derive(Resource)]
pub struct ToolMenu {
    /// Map of menu items
    menu_item: Entity,
}

impl ToolMenu {
    pub fn get(&self) -> Entity {
        return self.menu_item;
    }
}

impl FromWorld for ToolMenu {
    fn from_world(world: &mut World) -> Self {
        let menu_item = world
            .spawn(Menu {
                text: "Tool".to_string(),
            })
            .id();
        Self { menu_item }
    }
}

/// This resource provides the root entity for the tool menu
#[derive(Resource)]
pub struct ViewMenu {
    /// Map of menu items
    menu_item: Entity,
}

impl ViewMenu {
    pub fn get(&self) -> Entity {
        return self.menu_item;
    }
}

impl FromWorld for ViewMenu {
    fn from_world(world: &mut World) -> Self {
        let menu_item = world
            .spawn(Menu {
                text: "View".to_string(),
            })
            .id();
        Self { menu_item }
    }
}

#[non_exhaustive]
#[derive(Event)]
pub enum MenuEvent {
    MenuClickEvent(Entity),
}

impl MenuEvent {
    pub fn clicked(&self) -> bool {
        matches!(self, Self::MenuClickEvent(_))
    }

    pub fn source(&self) -> Entity {
        match self {
            Self::MenuClickEvent(entity) => *entity,
        }
    }
}

/// Helper function to render a submenu starting at the entity.
pub fn render_sub_menu(
    ui: &mut Ui,
    entity: &Entity,
    children: &Query<&Children>,
    menus: &Query<(&Menu, Entity)>,
    menu_items: &Query<(&mut MenuItem, Has<MenuDisabled>)>,
    extension_events: &mut EventWriter<MenuEvent>,
    skip_top_label: bool,
) {
    if let Ok((e, disabled)) = menu_items.get(*entity) {
        // Draw ui
        match e {
            MenuItem::Text(item) => {
                let mut button = Button::new(&item.text);
                if let Some(ref shortcut) = &item.shortcut {
                    button = button.shortcut_text(shortcut);
                }
                if ui.add_enabled(!disabled, button).clicked() {
                    extension_events.send(MenuEvent::MenuClickEvent(*entity));
                }
            }
            MenuItem::CheckBox(title, mut value) => {
                if ui
                    .add_enabled(!disabled, egui::Checkbox::new(&mut value, title))
                    .clicked()
                {
                    extension_events.send(MenuEvent::MenuClickEvent(*entity));
                }
            }
        }
        return;
    }

    let Ok((menu, _)) = menus.get(*entity) else {
        return;
    };

    if !skip_top_label {
        ui.menu_button(&menu.get(), |ui| {
            let Ok(child_items) = children.get(*entity) else {
                return;
            };

            for child in child_items.iter() {
                render_sub_menu(
                    ui,
                    child,
                    children,
                    menus,
                    menu_items,
                    extension_events,
                    false,
                );
            }
        });
    } else {
        let Ok(child_items) = children.get(*entity) else {
            return;
        };

        for child in child_items.iter() {
            render_sub_menu(
                ui,
                child,
                children,
                menus,
                menu_items,
                extension_events,
                false,
            );
        }
    }
}
