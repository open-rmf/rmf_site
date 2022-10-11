use bevy::{pbr::DirectionalLightShadowMap, prelude::*, render::render_resource::WgpuAdapterInfo};
use bevy_egui::EguiPlugin;
use clap::Parser;
use interaction::InteractionState;
use main_menu::MainMenuPlugin;
use rmf_site_format::{legacy::building_map::BuildingMap, Site};
use site::load::LoadSite;
use std::fs;
// use warehouse_generator::WarehouseGeneratorPlugin;
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
mod widgets;
use widgets::*;

mod demo_world;
mod shapes;

mod main_menu;
mod site;
// mod warehouse_generator;
mod interaction;

mod simulation_state;
mod site_asset_io;

use aabb::AabbUpdatePlugin;
use animate::AnimationPlugin;
use interaction::InteractionPlugin;
use site::SitePlugin;
use site_asset_io::SiteAssetIoPlugin;

#[derive(Clone, Eq, PartialEq, Debug, Hash)]
pub enum AppState {
    MainMenu,
    SiteEditor,
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

// todo: a way to accept arguments from JavaScript land
#[wasm_bindgen]
pub fn run_js() {
    run(vec!["web".to_string()]);
}

#[derive(Parser)]
struct Args {
    /// Filename of a Site or Building map
    filename: Option<String>,
}

fn argument_parser_system(
    mut load_site: EventWriter<LoadSite>,
    mut interaction_state: ResMut<State<InteractionState>>,
    args: Res<Args>,
    mut app_state: ResMut<State<AppState>>,
) {
    if args.filename.is_some() {
        let filename = args.filename.clone().unwrap(); //unwrap();
        let file_data = fs::read(filename).unwrap();
        match BuildingMap::from_bytes(&file_data) {
            Ok(building) => match building.to_site() {
                Ok(site) => {
                    println!("site loaded OK");
                    load_site.send(LoadSite {
                        site,
                        focus: true,
                        default_file: None,
                    });
                    match app_state.set(AppState::SiteEditor) {
                        Ok(_) => {
                            interaction_state.set(InteractionState::Enable).ok();
                        }
                        Err(err) => {
                            println!("Failed to enter traffic editor: {:?}", err);
                        }
                    }
                }
                Err(err) => {
                    println!("site parse error: {err:?}");
                }
            },
            Err(err) => {
                println!("building parse error {:?}", err);
            }
        }
    }
}

pub fn run(string_args: Vec<String>) {
    let mut app = App::new();

    #[cfg(target_arch = "wasm32")]
    {
        app.insert_resource(WindowDescriptor {
            title: "RMF Site Editor".to_string(),
            canvas: Some(String::from("#rmf_site_editor_canvas")),
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
            title: "RMF Site Editor".to_string(),
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
            group.add_before::<bevy::asset::AssetPlugin, _>(SiteAssetIoPlugin)
        })
        .add_plugin(AabbUpdatePlugin)
        .add_plugin(EguiPlugin)
        .add_plugin(KeyboardInputPlugin)
        .add_state(AppState::MainMenu)
        .add_plugin(MainMenuPlugin)
        // .add_plugin(WarehouseGeneratorPlugin)
        .add_plugin(SitePlugin)
        .add_plugin(InteractionPlugin)
        .add_plugin(StandardUiLayout)
        .add_plugin(AnimationPlugin)
        .insert_resource(Args::parse_from(string_args))
        .add_startup_system(argument_parser_system)
        .run();
}
