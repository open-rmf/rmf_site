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
use wasm_bindgen::JsValue;

pub mod aabb;
pub mod animate;

pub mod keyboard;
use keyboard::*;

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

pub mod view_menu;
use view_menu::*;

pub mod wireframe;
use wireframe::*;

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

use crate::main_menu::WebAutoLoad;

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

impl AppState {
    pub fn in_site_mode() -> impl Condition<()> {
        IntoSystem::into_system(|state: Res<State<AppState>>| match state.get() {
            AppState::SiteEditor | AppState::SiteVisualizer | AppState::SiteDrawingEditor => true,
            AppState::MainMenu | AppState::WorkcellEditor => false,
        })
    }

    pub fn in_displaying_mode() -> impl Condition<()> {
        IntoSystem::into_system(|state: Res<State<AppState>>| match state.get() {
            AppState::MainMenu => false,
            AppState::SiteEditor
            | AppState::SiteVisualizer
            | AppState::WorkcellEditor
            | AppState::SiteDrawingEditor => true,
        })
    }
}
#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
extern "C" {
    #[wasm_bindgen(js_namespace = console)]
    pub fn log(s: &str);

}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_js() {
    extern crate console_error_panic_hook;
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));

    run(vec!["web".to_owned()]);
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_js_with_data(buffer: JsValue, file_type: JsValue) {
    use js_sys::Uint8Array;

    #[cfg(target_arch = "wasm32")]
    log("Running RCC RMF Site Editor with map data");

    let array = Uint8Array::new(&buffer);
    let bytes: Vec<u8> = array.to_vec();

    let file_type: String = file_type.as_string().unwrap();

    let mut app: App = App::new();

    app.insert_resource(WebAutoLoad::file(bytes, file_type));
    app.add_plugins(SiteEditor);
    app.run();
}

pub fn run(command_line_args: Vec<String>) {
    let mut app: App = App::new();

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
                            title: "RCC RMF Site Editor".to_owned(),
                            canvas: Some(String::from("#rmf_site_editor_canvas")),
                            fit_canvas_to_parent: true,
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
                    .add_after::<bevy::asset::AssetPlugin, _>(SiteAssetIoPlugin),
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

        app.insert_resource(DirectionalLightShadowMap { size: 2048 })
            .add_state::<AppState>()
            .add_plugins((
                LogHistoryPlugin,
                AabbUpdatePlugin,
                EguiPlugin,
                KeyboardInputPlugin,
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
            // Note order matters, plugins that edit the menus must be initialized after the UI
            .add_plugins((
                ViewMenuPlugin,
                IssuePlugin,
                OSMViewPlugin,
                SiteWireframePlugin,
            ));
    }
}
