use bevy::prelude::*;
use librmf_site_editor::ui_command::TopLevelMenuExtensions;
use librmf_site_editor::ui_command::{EventHandle, MenuEvent};
use librmf_site_editor::SiteEditor;

#[derive(Debug, Default)]
struct MyMenuPlugin;

#[derive(Debug, Default, Resource)]
struct MyMenuHandler {
    event_handler: Option<EventHandle>,
}

/// Startup system to register menu.
fn init(mut data: ResMut<TopLevelMenuExtensions>, mut menu_handle: ResMut<MyMenuHandler>) {
    // This is all it takes to register a new menu item
    // We need to keep track of the event handler in order to make
    // sure that we can check the
    menu_handle.event_handler = Some(
        data.add_item(&"My Menu".to_string(), &"My Random Action".to_string())
            .unwrap(),
    );
}

/// Handler for menu item. All one needs tp dp is check that you recieve
/// an event that is of the same type as the one we are supposed to
/// handle.
fn watch_click(mut reader: EventReader<MenuEvent>, menu_handle: Res<MyMenuHandler>) {
    let Some(ref evt) = menu_handle.event_handler else {
        return;
    };
    for event in reader.iter() {
        if event.is_same(evt) {
            println!("Custom event clicked")
        }
    }
}

/// The actual plugin
impl Plugin for MyMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MyMenuHandler>()
            .add_startup_system(init)
            .add_system(watch_click);
    }
}

/// Lets embed site editor in our application with our own plugin
fn main() {
    App::new()
        .add_plugin(SiteEditor)
        .add_plugin(MyMenuPlugin)
        .run();
}
