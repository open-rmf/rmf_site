use bevy::prelude::*;
use librmf_site_editor::ui_command::{EventHandle, MenuEvent};
use librmf_site_editor::SiteEditor;
use librmf_site_editor::ui_command::TopLevelMenuExtensions;

#[derive(Debug, Default)]
struct MyMenuPlugin;

#[derive(Debug, Default, Resource)]
struct MyMenuHandler {
    event_handler: Option<EventHandle>
}

fn init(
    mut data: ResMut<TopLevelMenuExtensions>,
    mut menu_handle: ResMut<MyMenuHandler>) {
    menu_handle.event_handler = Some(data.add_item(
        &"My Menu".to_string(), 
        &"My Random Action".to_string()).unwrap());
}

fn watch_click(
    mut reader: EventReader<MenuEvent>,
    menu_handle: Res<MyMenuHandler>
) {
    let Some(ref evt) = menu_handle.event_handler else {
        return;
    };
    for event in reader.iter() {
        if event.is_same(evt) {
            println!("Custom event clicked")
        }
    }
}

impl Plugin for MyMenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MyMenuHandler>()
           .add_startup_system(init)
           .add_system(watch_click);
    }
}


fn main() {
    App::new()
        .add_plugin(SiteEditor)
        .add_plugin(MyMenuPlugin)
        .run();
}