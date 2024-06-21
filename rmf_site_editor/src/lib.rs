use bevy::{app::ScheduleRunnerPlugin, log::LogPlugin, pbr::DirectionalLightShadowMap, prelude::*};
use bevy_egui::EguiPlugin;
use main_menu::MainMenuPlugin;
// use warehouse_generator::WarehouseGeneratorPlugin;
#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
#[cfg(target_arch = "wasm32")]
use wasm_bindgen::prelude::*;

pub mod aabb;
pub mod animate;

pub mod asset_loaders;
use asset_loaders::*;

pub mod keyboard;
use keyboard::*;

pub mod widgets;
use widgets::*;
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
//pub mod urdf_loader;
use sdf_loader::*;

pub mod view_menu;
use view_menu::*;

pub mod wireframe;
use wireframe::*;

use aabb::AabbUpdatePlugin;
use animate::AnimationPlugin;
use interaction::InteractionPlugin;
use site::{OSMViewPlugin, SiteFileMenuPlugin, SitePlugin};
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
    /// Run in headless mode and export the loaded site to the requested path.
    #[cfg_attr(not(target_arch = "wasm32"), arg(long))]
    pub headless_export: Option<String>,
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
pub fn run_js() {
    extern crate console_error_panic_hook;
    std::panic::set_hook(Box::new(console_error_panic_hook::hook));
    run(vec!["web".to_owned()]);
}

pub fn run(command_line_args: Vec<String>) {
    let mut app = App::new();
    let mut headless_export = None;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let command_line_args = CommandLineArgs::parse_from(command_line_args);
        if let Some(path) = command_line_args.filename {
            app.insert_resource(Autoload::file(
                path.into(),
                command_line_args.import.map(Into::into),
            ));
        }
        headless_export = command_line_args.headless_export;
    }

    app.add_plugins(SiteEditor { headless_export });
    app.run();
}

#[derive(Default)]
pub struct SiteEditor {
    /// Contains Some(path) if the site editor is running in headless mode exporting its site.
    pub headless_export: Option<String>,
}

impl Plugin for SiteEditor {
    fn build(&self, app: &mut App) {
        let mut plugins = DefaultPlugins.build();
        let headless = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.headless_export.is_some()
            }
            #[cfg(target_arch = "wasm32")]
            {
                false
            }
        };
        plugins = if headless {
            plugins
                .set(WindowPlugin {
                    primary_window: None,
                    exit_condition: bevy::window::ExitCondition::DontExit,
                    close_when_requested: false,
                })
                .disable::<bevy::winit::WinitPlugin>()
        } else {
            plugins.set(WindowPlugin {
                primary_window: Some(Window {
                    title: "RMF Site Editor".to_owned(),
                    #[cfg(not(target_arch = "wasm32"))]
                    resolution: (1600., 900.).into(),
                    #[cfg(target_arch = "wasm32")]
                    canvas: Some(String::from("#rmf_site_editor_canvas")),
                    #[cfg(target_arch = "wasm32")]
                    fit_canvas_to_parent: true,
                    ..default()
                }),
                ..default()
            })
        };
        app.add_plugins((
            SiteAssetIoPlugin,
            plugins
                .disable::<LogPlugin>()
                .set(ImagePlugin {
                    default_sampler: SamplerDescriptor {
                        address_mode_u: AddressMode::Repeat,
                        address_mode_v: AddressMode::Repeat,
                        address_mode_w: AddressMode::Repeat,
                        ..Default::default()
                    }
                    .into(),
                })
                .set(RenderPlugin {
                    render_creation: WgpuSettings {
                        #[cfg(not(target_arch = "wasm32"))]
                        features: WgpuFeatures::POLYGON_MODE_LINE,
                        ..default()
                    }
                    .into(),
                    ..default()
                }),
        ));

        app.insert_resource(DirectionalLightShadowMap { size: 2048 })
            .add_state::<AppState>()
            .add_plugins((
                AssetLoadersPlugin,
                LogHistoryPlugin,
                AabbUpdatePlugin,
                EguiPlugin,
                KeyboardInputPlugin,
                MainMenuPlugin,
                WorkcellEditorPlugin,
                SitePlugin,
                InteractionPlugin {
                    headless: self.headless_export.is_some(),
                },
                StandardUiLayout {
                    headless: self.headless_export.is_some(),
                },
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
                SiteFileMenuPlugin,
            ));

        // Ref https://github.com/bevyengine/bevy/issues/10877. The default behavior causes issues
        // with events being accumulated when not read (i.e. scrolling mouse wheel on a UI widget).
        app.world
            .remove_resource::<bevy::ecs::event::EventUpdateSignal>();

        if let Some(path) = &self.headless_export {
            // We really don't need a high update rate here since we are IO bound, set a low rate
            // to save CPU.
            // TODO(luca) this still seems to take quite some time, check where the bottleneck is.
            app.add_plugins(ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_secs_f64(1.0 / 10.0),
            ));
            app.insert_resource(site::HeadlessSdfExportState::new(path));
            app.add_systems(Last, site::headless_sdf_export);
        }
    }
}
