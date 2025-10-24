//! minimal example showing each of the hooks

use bevy::prelude::*;
use bevy_dioxus_sync::plugins::DioxusPlugin;

use crate::bevy_scene_plugin::BevyScenePlugin;

mod bevy_scene_plugin;
mod ui;

fn main() {
    App::new()
        .add_plugins(DefaultPlugins)
        .add_plugins(DioxusPlugin {
            bevy_info_refresh_fps: 30,
        })
        .add_plugins(BevyScenePlugin)
        .run();
}
