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

use crate::{
    widgets::prelude::*, AppState, CreateNewWorkspace, SaveWorkspace, WorkspaceLoadingServices,
};

use bevy::ecs::query::Has;
use bevy::prelude::*;
use bevy_egui::egui::{self, Button, Ui};

use std::collections::HashSet;

/// Add the standard menu bar to the application.
#[derive(Default)]
pub struct MenuBarPlugin {}

impl Plugin for MenuBarPlugin {
    fn build(&self, app: &mut App) {
        let widget = PanelWidget::new(top_menu_bar, &mut app.world);
        app.world.spawn(widget);

        app.add_event::<MenuEvent>()
            .init_resource::<FileMenu>()
            .init_resource::<ToolMenu>()
            .init_resource::<ViewMenu>();
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
    Text(String),
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

/// Contains the states that the menu should be visualized in.
#[derive(Debug, Clone, Component, Deref, DerefMut)]
pub struct MenuVisualizationStates(pub HashSet<AppState>);

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
fn render_sub_menu(
    state: &State<AppState>,
    ui: &mut Ui,
    entity: &Entity,
    children: &Query<&Children>,
    menus: &Query<(&Menu, Entity)>,
    menu_items: &Query<(&mut MenuItem, Has<MenuDisabled>)>,
    menu_states: &Query<Option<&MenuVisualizationStates>>,
    extension_events: &mut EventWriter<MenuEvent>,
    skip_top_label: bool,
) {
    if let Some(states) = menu_states.get(*entity).ok().flatten() {
        if !states.contains(state.get()) {
            return;
        }
    }
    if let Ok((e, disabled)) = menu_items.get(*entity) {
        // Draw ui
        match e {
            MenuItem::Text(title) => {
                if ui.add_enabled(!disabled, Button::new(title)).clicked() {
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
                    state,
                    ui,
                    child,
                    children,
                    menus,
                    menu_items,
                    menu_states,
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
                state,
                ui,
                child,
                children,
                menus,
                menu_items,
                menu_states,
                extension_events,
                false,
            );
        }
    }
}

#[derive(SystemParam)]
struct MenuParams<'w, 's> {
    state: Res<'w, State<AppState>>,
    menus: Query<'w, 's, (&'static Menu, Entity)>,
    menu_items: Query<'w, 's, (&'static mut MenuItem, Has<MenuDisabled>)>,
    menu_states: Query<'w, 's, Option<&'static MenuVisualizationStates>>,
    extension_events: EventWriter<'w, MenuEvent>,
    view_menu: Res<'w, ViewMenu>,
}

fn top_menu_bar(
    In(input): In<PanelWidgetInput>,
    mut commands: Commands,
    mut new_workspace: EventWriter<CreateNewWorkspace>,
    mut save: EventWriter<SaveWorkspace>,
    load_workspace: Res<WorkspaceLoadingServices>,
    file_menu: Res<FileMenu>,
    top_level_components: Query<(), Without<Parent>>,
    children: Query<&Children>,
    mut menu_params: MenuParams,
) {
    egui::TopBottomPanel::top("top_panel").show(&input.context, |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.add(Button::new("New").shortcut_text("Ctrl+N")).clicked() {
                    new_workspace.send(CreateNewWorkspace);
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui
                        .add(Button::new("Save").shortcut_text("Ctrl+S"))
                        .clicked()
                    {
                        save.send(SaveWorkspace::new().to_default_file());
                    }
                    if ui
                        .add(Button::new("Save As").shortcut_text("Ctrl+Shift+S"))
                        .clicked()
                    {
                        save.send(SaveWorkspace::new().to_dialog());
                    }
                }
                if ui
                    .add(Button::new("Open").shortcut_text("Ctrl+O"))
                    .clicked()
                {
                    load_workspace.load_from_dialog(&mut commands);
                }

                render_sub_menu(
                    &menu_params.state,
                    ui,
                    &file_menu.get(),
                    &children,
                    &menu_params.menus,
                    &menu_params.menu_items,
                    &menu_params.menu_states,
                    &mut menu_params.extension_events,
                    true,
                );
            });
            ui.menu_button("View", |ui| {
                render_sub_menu(
                    &menu_params.state,
                    ui,
                    &menu_params.view_menu.get(),
                    &children,
                    &menu_params.menus,
                    &menu_params.menu_items,
                    &menu_params.menu_states,
                    &mut menu_params.extension_events,
                    true,
                );
            });

            for (_, entity) in menu_params.menus.iter().filter(|(_, entity)| {
                top_level_components.contains(*entity)
                    && (*entity != file_menu.get() && *entity != menu_params.view_menu.get())
            }) {
                render_sub_menu(
                    &menu_params.state,
                    ui,
                    &entity,
                    &children,
                    &menu_params.menus,
                    &menu_params.menu_items,
                    &menu_params.menu_states,
                    &mut menu_params.extension_events,
                    false,
                );
            }
        });
    });
}
