extern crate console_error_panic_hook;

use bevy::{
    log::LogPlugin, pbr::DirectionalLightShadowMap, prelude::*, render::{mesh::shape::Cube, renderer::RenderAdapterInfo},
};
use bevy_egui::EguiPlugin;

use main_menu::MainMenuPlugin;
use std::panic;

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

pub mod rcc;

use crate::main_menu::UploadData;
use crate::main_menu::WebAutoLoad;

use crate::rcc::{set_site_mode,is_site_in_view_mode};

// Define a struct to keep some information about our entity.
// Here it's an arbitrary movement speed, the spawn location, and a maximum distance from it.
#[derive(Component)]
struct Movable {
    spawn: Vec3,
    max_distance: f32,
    speed: f32,
}

// Implement a utility function for easier Movable struct creation.
impl Movable {
    fn new(spawn: Vec3) -> Self {
        Movable {
            spawn,
            max_distance: 5.0,
            speed: 2.0,
        }
    }
}

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

    #[wasm_bindgen(js_namespace = window)]
    pub fn save_site_map(id: &str, s: &str);

    #[wasm_bindgen(js_namespace = window)]
    pub fn save_nav_graph(id: &str, name: &str, s: &str);

    #[wasm_bindgen(js_namespace = window)]
    pub fn get_map_list() -> js_sys::Array;

    #[wasm_bindgen(js_namespace = window)]
    pub fn send_nav_graph_total(total: &js_sys::Number);

    #[wasm_bindgen(js_namespace = window)]
    pub fn site_mode() -> js_sys::JsString;

    #[wasm_bindgen(js_namespace = window)]
    pub fn get_robots_list() -> js_sys::Array;
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
pub fn run_js_with_data(buffer: JsValue, file_type: JsValue, building_id: JsValue) {
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    use js_sys::Uint8Array;

    #[cfg(target_arch = "wasm32")]
    log("Running RCC RMF Site Editor with map data");
    set_site_mode();
    

    let array = Uint8Array::new(&buffer);
    let bytes: Vec<u8> = array.to_vec();

    let file_type: String = file_type.as_string().unwrap();
    let building_id: String = building_id.as_string().unwrap();

    let mut app: App = App::new();

    app.insert_resource(WebAutoLoad::file(bytes, file_type));
    app.insert_resource(UploadData::new(building_id));
    app.add_plugins(SiteEditor);

    if is_site_in_view_mode() {
        app.add_systems(Startup, set_initial_robot_pose);
        app.add_systems(Update, update_robot_pose);
    }

    app.run();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn run_js_new_site(building_id: JsValue) {
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    #[cfg(target_arch = "wasm32")]
    log("Running RCC RMF Site Editor for new workspace");
    let building_id: String = building_id.as_string().unwrap();

    let mut app: App = App::new();

    app.insert_resource(UploadData::new(building_id));
    app.add_plugins(SiteEditor);
    app.run();
}

#[cfg(target_arch = "wasm32")]
#[wasm_bindgen]
pub fn send_robot_pose(robot_id: JsValue,robot_pose: js_sys::Object) {
    panic::set_hook(Box::new(console_error_panic_hook::hook));

    #[cfg(target_arch = "wasm32")]
    let robot_id: String = robot_id.as_string().unwrap();
    log(&robot_id);
   
    match rcc::parse_robot_pose(&robot_pose) {
        Ok(obj)=>{
            rcc::add_robot_pose_by_id(robot_id,obj);
        },
        Err(err)=>{
            #[cfg(target_arch = "wasm32")]
            {
                log( &format!("Error parsing  robot pose: {}", err));
            }
        }
    }

}

pub fn run(command_line_args: Vec<String>) {
    panic::set_hook(Box::new(console_error_panic_hook::hook));
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

fn set_initial_robot_pose(mut commands: Commands, mut meshes: ResMut<Assets<Mesh>>) {
    let entity_spawn = Vec3::ZERO;
    let robot_list = get_robots_list().clone();

    for i in 0..robot_list.length() {
        match rcc::parse_robot_data(&robot_list.get(i)) {
            Ok(robot_id)=>{
                rcc::add_robot_in_robot_list(robot_id, i);
            },
            Err(err)=>{
                #[cfg(target_arch = "wasm32")]
                {
                    log( &format!("Error parsing  robot list items JSON: {}", err));
                }
            }
        }

        commands.spawn((
            PbrBundle {
                mesh: meshes.add(Mesh::from(shape::Cube {
                    ..Default::default()
                })),
                transform: Transform::from_xyz(i as f32, 0.5, 0.0),
                ..default()
            },
            Movable::new(entity_spawn),
        ));
    }
}



fn update_robot_pose(mut cubes: Query<(&mut Transform, &mut Movable)>, timer: Res<Time>) {
    
    let mut index:u32 = 0;
    for (mut transform, mut cube) in &mut cubes {
        if let Some(robot_id) = rcc::get_robot_id(index) {
            
            if let Some(robot_pose) = rcc::get_robot_pose_by_id(&robot_id) {
                
                let direction = Vec3::new(robot_pose.x, robot_pose.y, 0.0);
                transform.translation += direction * cube.speed;
                // if (cube.spawn - transform.translation).length() > cube.max_distance {
                //     cube.speed *= -1.0;
                // }
            } else {
                log("unable to get robot pose");
            }
           
        } else {
            log("unable to get robot id");
        }

        index += 1;
    }
}