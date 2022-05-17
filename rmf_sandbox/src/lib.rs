use bevy::{pbr::DirectionalLightShadowMap, prelude::*};
use main_menu::MainMenuPlugin;
use traffic_editor::TrafficEditorPlugin;
use warehouse_generator::WarehouseGeneratorPlugin;
use wasm_bindgen::prelude::*;

// a few more imports needed for wasm32 only
#[cfg(target_arch = "wasm32")]
use bevy::{core::FixedTimestep, window::Windows};

extern crate web_sys;

mod camera_controls;
mod demo_world;

mod main_menu;
mod site_map;
mod traffic_editor;
mod warehouse_generator;

mod building_map;
mod lane;
mod level;
mod level_transform;
mod measurement;
mod model;
mod sandbox_asset_io;
mod vertex;
mod wall;

use camera_controls::CameraControlsPlugin;
use sandbox_asset_io::SandboxAssetIoPlugin;

use site_map::SiteMapPlugin;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    MainMenu,
    TrafficEditor,
    WarehouseGenerator,
}

#[cfg(target_arch = "wasm32")]
fn check_browser_window_size(mut windows: ResMut<Windows>) {
    let window = windows.get_primary_mut().unwrap();
    let wasm_window = web_sys::window().unwrap();
    let target_width = wasm_window.inner_width().unwrap().as_f64().unwrap() as f32;
    let target_height = wasm_window.inner_height().unwrap().as_f64().unwrap() as f32;
    let w_diff = (target_width - window.width()).abs();
    let h_diff = (target_height - window.height()).abs();

    if w_diff > 3. || h_diff > 3. {
        // web_sys::console::log_1(&format!("window = {} {} canvas = {} {}", window.width(), window.height(), target_width, target_height).into());
        window.set_resolution(target_width, target_height);
    }
}

#[wasm_bindgen]
pub fn run() {
    #[cfg(target_arch = "wasm32")]
    App::new()
        .insert_resource(WindowDescriptor {
            title: "RMF Sandbox".to_string(),
            canvas: Some(String::from("#rmf_sandbox_canvas")),
            //vsync: false,
            ..Default::default()
        })
        .add_plugins_with(DefaultPlugins, |group| {
            group.add_before::<bevy::asset::AssetPlugin, _>(SandboxAssetIoPlugin)
        })
        .insert_resource(DirectionalLightShadowMap { size: 1024 })
        .add_startup_system(setup)
        .add_plugin(SiteMapPlugin)
        .add_plugin(CameraControlsPlugin)
        .add_plugin(UIWidgetsPlugin)
        .add_plugin(WarehouseGeneratorPlugin)
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.5))
                .with_system(check_browser_window_size),
        )
        .run();

    #[cfg(not(target_arch = "wasm32"))]
    App::new()
        .insert_resource(WindowDescriptor {
            title: "RMF Sandbox".to_string(),
            width: 1600.,
            height: 900.,
            //vsync: false,
            ..Default::default()
        })
        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .add_plugins_with(DefaultPlugins, |group| {
            group.add_before::<bevy::asset::AssetPlugin, _>(SandboxAssetIoPlugin)
        })
        .add_plugin(bevy_egui::EguiPlugin)
        .add_state(AppState::MainMenu)
        //.add_plugin(FrameTimeDiagnosticsPlugin::default())
        //.add_plugin(LogDiagnosticsPlugin::default())
        //.insert_resource(Msaa { samples: 4})
        .add_plugin(MainMenuPlugin)
        .add_plugin(SiteMapPlugin)
        .add_plugin(CameraControlsPlugin)
        .add_plugin(TrafficEditorPlugin)
        .add_plugin(WarehouseGeneratorPlugin)
        .run();
}
