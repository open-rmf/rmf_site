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
    CreateNewWorkspace, FileEvents, LoadWorkspace, SaveWorkspace, VisibilityParameters,
};

use bevy::prelude::{EventWriter, Res, Children, Query, Entity, Parent, Without, Component, Resource, FromWorld, World, Plugin, App};
use bevy_egui::{
    egui::{self, epaint::ahash::HashSet, Button, Ui},
    EguiContext,
};

/// This component represents a menu. Menus and menu items 
/// can be arranged in trees using bevy's own parent-child system.
#[derive(Component)]
pub struct Menu {
    text: String,
}

impl Menu {
    /// Create a new menu from the title
    pub fn from_title(text: String) -> Self {
        Self {text}
    }

    /// Retrieve the menu name
    fn get(&self) -> String {
        self.text.clone()
    }
}

/// Create a new menu item
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


/// Helper function to render a submenu starting at the entity.
fn render_sub_menu(
    ui: &mut Ui,
    entity: &Entity,
    children: &Query<&Children>,
    menus: &Query<(&Menu, Entity)>,
    menu_items: &Query<&MenuItem>,
    extension_events: &mut EventWriter<MenuEvent>,
    skip_top_label: bool
) {
    if let Ok(e) =  menu_items.get(*entity) {
        // Draw ui
        match e {
            MenuItem::Text(title) => {
                if ui.add(Button::new(title)).clicked() {
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
            render_sub_menu(ui, child, children, menus, menu_items, extension_events, false);
        }
    });
}
else {
    let Ok(child_items) = children.get(*entity) else {
        return;
    };

    for child in child_items.iter() {
        render_sub_menu(ui, child, children, menus, menu_items, extension_events, false);
    }
}
}

pub fn top_menu_bar(
    egui_context: &mut EguiContext,
    file_events: &mut FileEvents,
    params: &mut VisibilityParameters,
    file_menu: &Res<FileMenu>,
    top_level_components: &Query<(), Without<Parent>>,
    children: &Query<&Children>,
    menus: &Query<(&Menu, Entity)>,
    menu_items: &Query<&MenuItem>,
    extension_events: &mut EventWriter<MenuEvent>,
) {
    egui::TopBottomPanel::top("top_panel").show(egui_context.ctx_mut(), |ui| {
        egui::menu::bar(ui, |ui| {
            ui.menu_button("File", |ui| {
                if ui.add(Button::new("New").shortcut_text("Ctrl+N")).clicked() {
                    file_events.new_workspace.send(CreateNewWorkspace);
                }
                #[cfg(not(target_arch = "wasm32"))]
                {
                    if ui
                        .add(Button::new("Save").shortcut_text("Ctrl+S"))
                        .clicked()
                    {
                        file_events
                            .save
                            .send(SaveWorkspace::new().to_default_file());
                    }
                    if ui
                        .add(Button::new("Save As").shortcut_text("Ctrl+Shift+S"))
                        .clicked()
                    {
                        file_events.save.send(SaveWorkspace::new().to_dialog());
                    }
                }
                if ui
                    .add(Button::new("Open").shortcut_text("Ctrl+O"))
                    .clicked()
                {
                    file_events.load_workspace.send(LoadWorkspace::Dialog);
                }
                
                render_sub_menu(ui, &file_menu.get(), children, menus, menu_items, extension_events, true);
            });
            ui.menu_button("View", |ui| {
                if ui
                    .checkbox(&mut params.resources.doors.0.clone(), "Doors")
                    .clicked()
                {
                    params.events.doors.send((!params.resources.doors.0).into());
                }
                if ui
                    .checkbox(&mut params.resources.floors.0.clone(), "Floors")
                    .clicked()
                {
                    params
                        .events
                        .floors
                        .send((!params.resources.floors.0).into());
                }
                if ui
                    .checkbox(&mut params.resources.lanes.0.clone(), "Lanes")
                    .clicked()
                {
                    params.events.lanes.send((!params.resources.lanes.0).into());
                }
                if ui
                    .checkbox(&mut params.resources.lift_cabins.0.clone(), "Lifts")
                    .clicked()
                {
                    // Bundle cabin and doors together
                    params
                        .events
                        .lift_cabins
                        .send((!params.resources.lift_cabins.0).into());
                    params
                        .events
                        .lift_cabin_doors
                        .send((!params.resources.lift_cabin_doors.0).into());
                }
                if ui
                    .checkbox(&mut params.resources.locations.0.clone(), "Locations")
                    .clicked()
                {
                    params
                        .events
                        .locations
                        .send((!params.resources.locations.0).into());
                }
                if ui
                    .checkbox(&mut params.resources.fiducials.0.clone(), "Fiducials")
                    .clicked()
                {
                    params
                        .events
                        .fiducials
                        .send((!params.resources.fiducials.0).into());
                }
                if ui
                    .checkbox(&mut params.resources.constraints.0.clone(), "Constraints")
                    .clicked()
                {
                    params
                        .events
                        .constraints
                        .send((!params.resources.constraints.0).into());
                }
                if ui
                    .checkbox(&mut params.resources.measurements.0.clone(), "Measurements")
                    .clicked()
                {
                    params
                        .events
                        .measurements
                        .send((!params.resources.measurements.0).into());
                }
                if ui
                    .checkbox(&mut params.resources.models.0.clone(), "Models")
                    .clicked()
                {
                    params
                        .events
                        .models
                        .send((!params.resources.models.0).into());
                }
                if ui
                    .checkbox(&mut params.resources.walls.0.clone(), "Walls")
                    .clicked()
                {
                    params.events.walls.send((!params.resources.walls.0).into());
                }
            });

            for (_, entity) in menus.iter().filter(|(_, entity)| {
                top_level_components.contains(*entity) && *entity != file_menu.get()
            }) {
                render_sub_menu(ui, &entity, children, menus, menu_items, extension_events, false);
            };
        });
    });
}
