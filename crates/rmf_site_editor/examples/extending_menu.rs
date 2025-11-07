use bevy::prelude::*;
use rmf_site_editor::SiteEditor;
use rmf_site_egui::*;

#[derive(Debug, Default)]
struct MyMenuPlugin;

#[derive(Debug, Resource)]
struct MyMenuHandler {
    unique_export: Entity,
    custom_nested_menu: Entity,
}

impl FromWorld for MyMenuHandler {
    fn from_world(world: &mut World) -> Self {
        // This is all it takes to register a new menu item
        // We need to keep track of the entity in order to make
        // sure that we can check the callback
        let unique_export = world.spawn(MenuItem::Text("My unique export".into())).id();

        // Make it a child of the "File Menu"
        let file_header = world.resource::<FileMenu>().get();
        world.entity_mut(file_header).add_children(&[unique_export]);

        // For top level menus simply spawn a menu with no parent
        let menu = world
            .spawn(Menu::from_title("My Awesome Menu".to_string()))
            .id();

        // We can use bevy's parent-child system to handle nesting
        let sub_menu = world
            .spawn(Menu::from_title("My Awesome sub menu".to_string()))
            .id();
        world.entity_mut(menu).add_children(&[sub_menu]);

        // Finally we can create a custom action
        let custom_nested_menu = world.spawn(MenuItem::Text("My Awesome Action".into())).id();
        world
            .entity_mut(sub_menu)
            .add_children(&[custom_nested_menu]);

        // Track the entity so that we know when to handle events from it in
        Self {
            unique_export,
            custom_nested_menu,
        }
    }
}

/// Handler for unique export menu item. All one needs to do is check that you recieve
/// an event that is of the same type as the one we are supposed to
/// handle.
fn watch_unique_export_click(mut reader: EventReader<MenuEvent>, menu_handle: Res<MyMenuHandler>) {
    for event in reader.read() {
        if event.clicked() && event.source() == menu_handle.unique_export {
            println!("Doing our epic export");
        }
    }
}

/// Handler for unique export menu item. All one needs to do is check that you recieve
/// an event that is of the same type as the one we are supposed to
/// handle.
fn watch_submenu_click(mut reader: EventReader<MenuEvent>, menu_handle: Res<MyMenuHandler>) {
    for event in reader.read() {
        if event.clicked() && event.source() == menu_handle.custom_nested_menu {
            println!("Submenu clicked");
        }
    }
}

/// The actual plugin
impl Plugin for MyMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MyMenuHandler>()
            .add_systems(Update, (watch_unique_export_click, watch_submenu_click));
    }
}

/// Lets embed site editor in our application with our own plugin
fn main() {
    App::new()
        .add_plugins((SiteEditor::default(), MyMenuPlugin))
        .run();
}
