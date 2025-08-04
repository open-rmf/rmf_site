use bevy::{
    app::ScheduleRunnerPlugin, asset::UnapprovedPathMode, log::LogPlugin,
    pbr::DirectionalLightShadowMap, prelude::*,
};
use bevy_egui::EguiPlugin;
#[cfg(not(target_arch = "wasm32"))]
use clap::Parser;
use main_menu::MainMenuPlugin;

pub mod aabb;
pub mod autoload;
pub use autoload::*;

pub mod asset_loaders;
use asset_loaders::*;

pub mod exit_confirmation;
use exit_confirmation::ExitConfirmationPlugin;

// Bevy plugins that are public dependencies, mixing versions won't work for downstream users
pub use bevy_egui;

pub mod keyboard;
use keyboard::*;

pub mod widgets;
use rmf_site_animate::VisualCueAnimationsPlugin;
use widgets::*;
pub mod occupancy;
use occupancy::OccupancyPlugin;
pub mod issue;
use issue::*;

pub mod demo_world;
pub mod log;
mod recency;
use log::LogHistoryPlugin;
use recency::*;

pub mod interaction;
pub mod main_menu;
pub mod site;

pub mod workspace;
use workspace::*;

pub mod sdf_loader;

pub mod site_asset_io;
use sdf_loader::*;

pub mod wireframe;
use wireframe::*;

use aabb::AabbUpdatePlugin;
use interaction::InteractionPlugin;
use site::{OSMViewPlugin, SitePlugin};
use site_asset_io::SiteAssetIoPlugin;

pub mod mapf_rse;
use mapf_rse::MapfRsePlugin;

pub mod osm_slippy_map;
use bevy::render::{
    batching::gpu_preprocessing::{GpuPreprocessingMode, GpuPreprocessingSupport},
    render_resource::{AddressMode, SamplerDescriptor},
    settings::{WgpuFeatures, WgpuSettings},
    RenderApp, RenderPlugin,
};
pub use osm_slippy_map::*;

#[derive(Debug, Clone, Copy, Resource)]
pub struct DebugMode(pub bool);

impl FromWorld for DebugMode {
    fn from_world(_: &mut World) -> Self {
        DebugMode(false)
    }
}

#[cfg_attr(not(target_arch = "wasm32"), derive(Parser))]
pub struct CommandLineArgs {
    /// Filename of a Site (.site.ron / .site.json) or Building (.building.yaml) file to load.
    /// Exclude this argument to get the main menu.
    pub filename: Option<String>,
    /// Name of a Site (.site.json or .site.ron) file to import on top of the base FILENAME.
    #[cfg_attr(not(target_arch = "wasm32"), arg(short, long))]
    pub import: Option<String>,
    /// Run in headless mode and export the loaded site to the requested path.
    /// This requires you to specify FILENAME, and it can be used with export_nav.
    #[cfg_attr(not(target_arch = "wasm32"), arg(long))]
    pub export_sdf: Option<String>,
    /// Run in headless mode and export the nav graphs to the requested path.
    /// This requires you to specify FILENAME, and it can be used with export_sdf.
    #[cfg_attr(not(target_arch = "wasm32"), arg(long))]
    pub export_nav: Option<String>,
}

#[derive(Clone, Default, Eq, PartialEq, Debug, Hash, States)]
pub enum AppState {
    #[default]
    MainMenu,
    SiteEditor,
    SiteVisualizer,
    SiteDrawingEditor,
}

impl AppState {
    pub fn in_displaying_mode() -> impl Condition<()> {
        IntoSystem::into_system(|state: Res<State<AppState>>| match state.get() {
            AppState::MainMenu => false,
            AppState::SiteEditor | AppState::SiteVisualizer | AppState::SiteDrawingEditor => true,
        })
    }
}

pub fn run(command_line_args: Vec<String>) {
    let mut app = App::new();
    let mut _export_sdf = None;
    let mut _export_nav = None;

    #[cfg(not(target_arch = "wasm32"))]
    {
        let command_line_args = CommandLineArgs::parse_from(command_line_args);
        if let Some(path) = command_line_args.filename {
            app.insert_resource(Autoload::file(
                path.into(),
                command_line_args.import.map(Into::into),
            ));
        }
        _export_sdf = command_line_args.export_sdf;
        _export_nav = command_line_args.export_nav;
    }

    app.add_plugins(
        SiteEditor::default()
            .export_sdf(_export_sdf)
            .export_nav(_export_nav),
    );
    app.run();
}

#[derive(Default)]
pub struct SiteEditor {
    /// Contains Some(path) if the site editor is running in headless mode
    /// exporting its site as an SDF.
    export_sdf: Option<String>,
    /// Contains Some(path) if the site editor is running in headless mode
    /// exporting its nav graphs.
    export_nav: Option<String>,
    /// Contains Some(path) if the site editor is running in headless mode and
    /// saving the contents of the site as a new path. This is primarily used
    /// for unit testing.
    save_as_path: Option<String>,
}

impl SiteEditor {
    pub fn new() -> Self {
        Self::default()
    }

    pub fn export_sdf(mut self, export_to_file: Option<String>) -> Self {
        self.export_sdf = export_to_file;
        self
    }

    pub fn export_nav(mut self, export_to_file: Option<String>) -> Self {
        self.export_nav = export_to_file;
        self
    }

    pub fn save_as_path(mut self, path: Option<String>) -> Self {
        self.save_as_path = path;
        self
    }

    pub fn is_headless(&self) -> bool {
        self.is_headless_export()
    }

    // This is a separate function from is_headless just in case there are other
    // reasons to run headless in the future, e.g. headless simulation.
    pub fn is_headless_export(&self) -> bool {
        self.export_sdf.is_some() || self.export_nav.is_some() || self.save_as_path.is_some()
    }
}

impl Plugin for SiteEditor {
    fn build(&self, app: &mut App) {
        let mut plugins = DefaultPlugins
            .set(AssetPlugin {
                unapproved_path_mode: UnapprovedPathMode::Deny,
                ..Default::default()
            })
            .build();

        let headless = {
            #[cfg(not(target_arch = "wasm32"))]
            {
                self.is_headless()
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
                close_when_requested: false,
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
            .init_state::<AppState>()
            .add_plugins((
                AssetLoadersPlugin,
                LogHistoryPlugin,
                AabbUpdatePlugin,
                EguiPlugin {
                    enable_multipass_for_primary_context: false,
                },
                ExitConfirmationPlugin,
                EditorInputPlugin,
                SitePlugin,
                InteractionPlugin::new().headless(self.is_headless()),
                VisualCueAnimationsPlugin,
                OccupancyPlugin,
                WorkspacePlugin,
                IssuePlugin,
                bevy_impulse::ImpulsePlugin::default(),
            ));

        if self.is_headless() {
            // Turn off GPU preprocessing in headless mode so that this can
            // work in GitHub CI. Without this, we were encountering this error:
            // https://github.com/bevyengine/bevy/issues/18932
            app.sub_app_mut(RenderApp)
                .insert_resource(GpuPreprocessingSupport {
                    max_supported_mode: GpuPreprocessingMode::None,
                });
        } else
        /* with rendering */
        {
            app.add_plugins((StandardUiPlugin::default(), MainMenuPlugin))
                // Note order matters, plugins that edit the menus must be initialized after the UI
                .add_plugins((site::ViewMenuPlugin, OSMViewPlugin, SiteWireframePlugin))
                .add_plugins(MapfRsePlugin::default());
        }

        if self.is_headless_export() {
            // We really don't need a high update rate here since we are IO bound, set a low rate
            // to save CPU.
            // TODO(luca) this still seems to take quite some time, check where the bottleneck is.
            app.add_plugins(ScheduleRunnerPlugin::run_loop(
                std::time::Duration::from_secs_f64(1.0 / 10.0),
            ));
            app.insert_resource(site::HeadlessExportState::new(
                self.export_sdf.clone(),
                self.export_nav.clone(),
                self.save_as_path.clone(),
            ));
            app.add_systems(Last, site::headless_export);
        }
    }
}
