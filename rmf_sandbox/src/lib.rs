use bevy::{pbr::DirectionalLightShadowMap, prelude::*, render::render_resource::WgpuAdapterInfo};
use bevy_egui::EguiPlugin;
use main_menu::MainMenuPlugin;
use traffic_editor::TrafficEditorPlugin;
use warehouse_generator::WarehouseGeneratorPlugin;
use wasm_bindgen::prelude::*;

// a few more imports needed for wasm32 only
#[cfg(target_arch = "wasm32")]
use bevy::{time::FixedTimestep, window::Windows};

extern crate web_sys;

mod settings;
use settings::*;

mod widgets;

mod camera_controls;
mod demo_world;
mod despawn;
mod save_load;
mod spawner;

mod main_menu;
mod site_map;
mod traffic_editor;
mod warehouse_generator;

mod basic_components;
mod building_map;
mod crowd_sim;
mod door;
mod fiducial;
mod floor;
mod interaction;
mod lane;
mod level;
mod level_transform;
mod lift;
mod light;
mod measurement;
mod model;
mod physical_camera;
mod rbmf;
mod sandbox_asset_io;
mod utils;
mod vertex;
mod wall;

use camera_controls::CameraControlsPlugin;
use despawn::DespawnPlugin;
use sandbox_asset_io::SandboxAssetIoPlugin;
use save_load::SaveLoadPlugin;
use spawner::SpawnerPlugin;

use site_map::SiteMapPlugin;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    MainMenu,
    TrafficEditor,
    WarehouseGenerator,
}

pub struct OpenedMapFile(std::path::PathBuf);

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

fn init_settings(mut settings: ResMut<Settings>, adapter_info: Res<WgpuAdapterInfo>) {
    // todo: be more sophisticated
    let is_elite = adapter_info.name.contains("NVIDIA");
    if is_elite {
        settings.graphics_quality = GraphicsQuality::Ultra;
    } else {
        settings.graphics_quality = GraphicsQuality::Low;
    }
}

#[wasm_bindgen]
pub fn run() {
    let mut app = App::new();

    #[cfg(target_arch = "wasm32")]
    {
        app.insert_resource(WindowDescriptor {
            title: "RMF Sandbox".to_string(),
            canvas: Some(String::from("#rmf_sandbox_canvas")),
            //vsync: false,
            ..Default::default()
        })
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.5))
                .with_system(check_browser_window_size),
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        app.insert_resource(WindowDescriptor {
            title: "RMF Sandbox".to_string(),
            width: 1600.,
            height: 900.,
            //vsync: false,
            ..Default::default()
        });
    }

    app.init_resource::<Settings>()
        .add_startup_system(init_settings)
        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .add_plugins_with(DefaultPlugins, |group| {
            group.add_before::<bevy::asset::AssetPlugin, _>(SandboxAssetIoPlugin)
        })
        .add_plugin(EguiPlugin)
        .add_state(AppState::MainMenu)
        //.add_plugin(FrameTimeDiagnosticsPlugin::default())
        //.add_plugin(LogDiagnosticsPlugin::default())
        //.insert_resource(Msaa { samples: 4})
        .add_plugin(MainMenuPlugin)
        .add_plugin(SiteMapPlugin)
        .add_plugin(CameraControlsPlugin)
        .add_plugin(TrafficEditorPlugin)
        .add_plugin(WarehouseGeneratorPlugin)
        .add_plugin(DespawnPlugin)
        .add_plugin(SpawnerPlugin)
        .add_plugin(SaveLoadPlugin)
        .run();
}
