use bevy::{
    log::LogPlugin, pbr::DirectionalLightShadowMap, prelude::*, render::renderer::RenderAdapterInfo,
};
use bevy_egui::EguiPlugin;
use main_menu::MainMenuPlugin;
// use warehouse_generator::WarehouseGeneratorPlugin;
#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
use wasm_bindgen::prelude::*;

// a few more imports needed for wasm32 only
#[cfg(target_arch = "wasm32")]
use bevy::{time::FixedTimestep, window::Windows};

extern crate web_sys;

mod aabb;
mod animate;

mod keyboard;
use keyboard::*;

mod settings;
use settings::*;
mod save;
use save::*;
mod widgets;
use widgets::*;

pub mod occupancy;
use occupancy::OccupancyPlugin;

mod demo_world;
mod recency;
mod shapes;

mod main_menu;
use main_menu::Autoload;
mod site;
// mod warehouse_generator;
mod workcell;
use workcell::WorkcellEditorPlugin;
mod interaction;

mod workspace;
use workspace::*;

mod sdf_loader;

mod site_asset_io;
mod urdf_loader;
use sdf_loader::*;

use aabb::AabbUpdatePlugin;
use animate::AnimationPlugin;
use interaction::InteractionPlugin;
use site::SitePlugin;
use site_asset_io::SiteAssetIoPlugin;

#[cfg_attr(not(target_arch = "wasm32"), derive(Parser))]
struct CommandLineArgs {
    /// Filename of a Site (.site.ron) or Building (.building.yaml) file to load.
    /// Exclude this argument to get the main menu.
    filename: Option<String>,
    /// Name of a Site (.site.ron) file to import on top of the base FILENAME.
    #[cfg_attr(not(target_arch = "wasm32"), arg(short, long))]
    import: Option<String>,
}

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    MainMenu,
    SiteEditor,
    //WarehouseGenerator,
    WorkcellEditor,
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

fn init_settings(mut settings: ResMut<Settings>, adapter_info: Res<RenderAdapterInfo>) {
    // todo: be more sophisticated
    let is_elite = adapter_info.name.contains("NVIDIA");
    if is_elite {
        settings.graphics_quality = GraphicsQuality::Ultra;
    } else {
        settings.graphics_quality = GraphicsQuality::Low;
    }
}

#[wasm_bindgen]
pub fn run_js() {
    run(vec!["web".to_owned()]);
}

pub fn run(command_line_args: Vec<String>) {
    let mut app = App::new();

    #[cfg(not(target_arch = "wasm32"))]
    {
        let command_line_args = CommandLineArgs::parse_from(command_line_args);
        if let Some(path) = command_line_args.filename {
            app.insert_resource(Autoload::file(
                path.into(),
                command_line_args.import.map(Into::into),
            ));
        }
    }

    #[cfg(target_arch = "wasm32")]
    {
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<LogPlugin>()
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "RMF Site Editor".to_owned(),
                        canvas: Some(String::from("#rmf_site_editor_canvas")),
                        ..default()
                    },
                    ..default()
                })
                .add_after::<bevy::asset::AssetPlugin, _>(SiteAssetIoPlugin),
        )
        .add_system_set(
            SystemSet::new()
                .with_run_criteria(FixedTimestep::step(0.5))
                .with_system(check_browser_window_size),
        );
    }

    #[cfg(not(target_arch = "wasm32"))]
    {
        app.add_plugins(
            DefaultPlugins
                .build()
                .disable::<LogPlugin>()
                .set(WindowPlugin {
                    window: WindowDescriptor {
                        title: "RMF Site Editor".to_owned(),
                        width: 1600.,
                        height: 900.,
                        ..default()
                    },
                    ..default()
                })
                .add_after::<bevy::asset::AssetPlugin, _>(SiteAssetIoPlugin),
        );
    }

    app.init_resource::<Settings>()
        .add_startup_system(init_settings)
        .insert_resource(DirectionalLightShadowMap { size: 2048 })
        .add_plugin(AabbUpdatePlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(KeyboardInputPlugin)
        .add_plugin(SavePlugin)
        .add_plugin(SdfPlugin)
        .add_state(AppState::MainMenu)
        .add_plugin(MainMenuPlugin)
        // .add_plugin(WarehouseGeneratorPlugin)
        .add_plugin(WorkcellEditorPlugin)
        .add_plugin(SitePlugin)
        .add_plugin(InteractionPlugin)
        .add_plugin(StandardUiLayout)
        .add_plugin(AnimationPlugin)
        .add_plugin(OccupancyPlugin)
        .add_plugin(WorkspacePlugin)
        .run();
}
