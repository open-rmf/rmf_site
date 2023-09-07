use bevy::{
    log::LogPlugin, pbr::DirectionalLightShadowMap, prelude::*, render::renderer::RenderAdapterInfo,
};
use bevy_egui::EguiPlugin;
use main_menu::MainMenuPlugin;
// use warehouse_generator::WarehouseGeneratorPlugin;
#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

// a few more imports needed for wasm32 only
#[cfg(target_arch = "wasm32")]
use bevy::{time::common_conditions::on_timer, window::PrimaryWindow};

extern crate web_sys;

pub mod aabb;
pub mod animate;

pub mod keyboard;
use keyboard::*;

pub mod settings;
use settings::*;
pub mod save;
use save::*;
pub mod widgets;
use widgets::{menu_bar::MenuPluginManager, *};

pub mod occupancy;
use occupancy::OccupancyPlugin;
pub mod issue;
use issue::*;

pub mod demo_world;
pub mod log;
mod recency;
use recency::*;
mod shapes;
use log::LogHistoryPlugin;

pub mod main_menu;
use main_menu::Autoload;
pub mod site;
// mod warehouse_generator;
pub mod workcell;
use workcell::WorkcellEditorPlugin;
pub mod interaction;

pub mod workspace;
use workspace::*;

pub mod sdf_loader;

pub mod site_asset_io;
pub mod urdf_loader;
use sdf_loader::*;

use aabb::AabbUpdatePlugin;
use animate::AnimationPlugin;
use interaction::InteractionPlugin;
use site::{OSMViewPlugin, SitePlugin};
use site_asset_io::SiteAssetIoPlugin;

pub mod osm_slippy_map;
use bevy::render::{
    render_resource::{AddressMode, SamplerDescriptor},
    settings::{WgpuFeatures, WgpuSettings},
    RenderPlugin,
};
pub use osm_slippy_map::*;

#[cfg_attr(not(target_arch = "wasm32"), derive(Parser))]
pub struct CommandLineArgs {
    /// Filename of a Site (.site.ron) or Building (.building.yaml) file to load.
    /// Exclude this argument to get the main menu.
    pub filename: Option<String>,
    /// Name of a Site (.site.ron) file to import on top of the base FILENAME.
    #[cfg_attr(not(target_arch = "wasm32"), arg(short, long))]
    pub import: Option<String>,
}

#[derive(Clone, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum AppState {
    #[default]
    MainMenu,
    SiteEditor,
    SiteVisualizer,
    //WarehouseGenerator,
    WorkcellEditor,
    SiteDrawingEditor,
}

pub struct OpenedMapFile(std::path::PathBuf);

#[cfg(target_arch = "wasm32")]
fn check_browser_window_size(mut windows: Query<&mut Window, With<PrimaryWindow>>) {
    let mut window = windows.get_single_mut().unwrap();
    let wasm_window = web_sys::window().unwrap();
    let target_width = wasm_window.inner_width().unwrap().as_f64().unwrap() as f32;
    let target_height = wasm_window.inner_height().unwrap().as_f64().unwrap() as f32;
    let w_diff = (target_width - window.width()).abs();
    let h_diff = (target_height - window.height()).abs();

    if w_diff > 3. || h_diff > 3. {
        // web_sys::console::log_1(&format!("window = {} {} canvas = {} {}", window.width(), window.height(), target_width, target_height).into());
        window.resolution = (target_width, target_height).into();
    }
}

pub fn init_settings(mut settings: ResMut<Settings>, adapter_info: Res<RenderAdapterInfo>) {
    // todo: be more sophisticated
    let is_elite = adapter_info.name.contains("NVIDIA");
    if is_elite {
        settings.graphics_quality = GraphicsQuality::Ultra;
    } else {
        settings.graphics_quality = GraphicsQuality::Low;
    }
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_js() {
    extern crate console_error_panic_hook;
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
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

    app.add_plugins(SiteEditor);
    app.run();
}

pub struct SiteEditor;

impl Plugin for SiteEditor {
    fn build(&self, app: &mut App) {
        #[cfg(target_arch = "wasm32")]
        {
            app.add_plugins(
                DefaultPlugins
                    .build()
                    .disable::<LogPlugin>()
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title: "RMF Site Editor".to_owned(),
                            canvas: Some(String::from("#rmf_site_editor_canvas")),
                            ..default()
                        }),
                        ..default()
                    })
                    .set(ImagePlugin {
                        default_sampler: SamplerDescriptor {
                            address_mode_u: AddressMode::Repeat,
                            address_mode_v: AddressMode::Repeat,
                            address_mode_w: AddressMode::Repeat,
                            ..Default::default()
                        },
                    })
                    .set(RenderPlugin {
                        wgpu_settings: WgpuSettings {
                            features: WgpuFeatures::POLYGON_MODE_LINE,
                            ..default()
                        },
                        ..default()
                    })
                    .add_after::<bevy::asset::AssetPlugin, _>(SiteAssetIoPlugin),
            )
            .add_systems(
                Update,
                check_browser_window_size.run_if(on_timer(std::time::Duration::from_secs_f32(0.5))),
            );
        }

        #[cfg(not(target_arch = "wasm32"))]
        {
            app.add_plugins(
                DefaultPlugins
                    .build()
                    .disable::<LogPlugin>()
                    .set(WindowPlugin {
                        primary_window: Some(Window {
                            title: "RMF Site Editor".to_owned(),
                            resolution: (1600., 900.).into(),
                            ..default()
                        }),
                        ..default()
                    })
                    .set(ImagePlugin {
                        default_sampler: SamplerDescriptor {
                            address_mode_u: AddressMode::Repeat,
                            address_mode_v: AddressMode::Repeat,
                            address_mode_w: AddressMode::Repeat,
                            ..Default::default()
                        },
                    })
                    .set(RenderPlugin {
                        wgpu_settings: WgpuSettings {
                            features: WgpuFeatures::POLYGON_MODE_LINE,
                            ..default()
                        },
                        ..default()
                    })
                    .add_after::<bevy::asset::AssetPlugin, _>(SiteAssetIoPlugin),
            );
        }
        app.init_resource::<Settings>()
            .add_systems(Startup, init_settings)
            .insert_resource(DirectionalLightShadowMap { size: 2048 })
            .add_state::<AppState>()
            .add_plugins((
                LogHistoryPlugin,
                AabbUpdatePlugin,
                EguiPlugin,
                KeyboardInputPlugin,
                SavePlugin,
                SdfPlugin,
                MainMenuPlugin,
                WorkcellEditorPlugin,
                SitePlugin,
                InteractionPlugin,
                StandardUiLayout,
                AnimationPlugin,
                OccupancyPlugin,
                WorkspacePlugin,
            ))
            // Note order matters, issue and OSMView plugins must be initialized after the UI
            .add_plugins((IssuePlugin, OSMViewPlugin));
    }
}
