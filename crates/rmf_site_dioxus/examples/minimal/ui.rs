use bevy::pbr::{MeshMaterial3d, StandardMaterial};
use bevy_color::Color;
use bevy_dioxus_sync::{
    hooks::{
        asset_single::hook::use_bevy_asset_singleton,
        component_single::hook::use_bevy_component_singleton,
    },
    resource_sync::hook::use_bevy_resource,
    traits::DioxusElementMarker,
};
use bevy_transform::components::Transform;
use dioxus::prelude::*;

use crate::bevy_scene_plugin::{CubeRotationSpeed, CubeTranslationSpeed, DynamicCube, FPS};

#[derive(Debug)]
pub struct AppUi;

impl DioxusElementMarker for AppUi {
    fn element(&self) -> Element {
        app_ui()
    }
}

pub fn app_ui() -> Element {
    let fps = use_bevy_resource::<FPS>();
    let cube_color = use_bevy_asset_singleton::<MeshMaterial3d<StandardMaterial>, _, DynamicCube>();
    let cube_rotation_speed = use_bevy_resource::<CubeRotationSpeed>();
    let cube_translation_speed = use_bevy_resource::<CubeTranslationSpeed>();
    let cube_transform = use_bevy_component_singleton::<Transform, DynamicCube>();

    const DEMO_CSS: Asset = asset!("./examples/minimal/ui.css");
    rsx! {
        document::Stylesheet { href: DEMO_CSS }
        style { {include_str!("./ui.css")} }
        div {
            id: "panel",
            class: "catch-events",
            div {
                id: "title",
                h1 { "Dioxus In Bevy Example" }
            }
            div {
                id: "buttons",
                button {
                    id: "button-red",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.peek().set_asset(StandardMaterial::from_color(Color::srgba(1.0, 0.0, 0.0, 1.0)))
                    },
                }
                button {
                    id: "button-green",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.peek().set_asset(StandardMaterial::from_color(Color::srgba(0.0, 1.0, 0.0, 1.0)))
                    },
                }
                button {
                    id: "button-blue",
                    class: "color-button",
                    onclick: move |_| {
                        cube_color.peek().set_asset(StandardMaterial::from_color(Color::srgba(0.0, 0.0, 1.0, 1.0)))
                    },
                }
            }
            div {
                id: "cube-rotation",
                label {
                    {"Cube Rotation: ".to_string() + &cube_transform.read().read_component().map(|n| format!("{:#}", n.rotation)).unwrap_or("???".to_string())}
                }
            }
            div {
                id: "translation-speed-control",
                label { "Translation Speed:" }
                input {
                    r#type: "number",
                    min: "0.0",
                    max: "10.0",
                    step: "0.1",
                    value: "{cube_translation_speed}",
                    oninput: move |event| {
                        if let Ok(speed) = event.value().parse::<f32>() {
                            cube_translation_speed.peek().set_resource(CubeTranslationSpeed(speed));
                        }
                    }
                }
            }
            div {
                id: "rotation-speed-control",
                label { "Rotation Speed:" }
                input {
                    r#type: "number",
                    min: "0.0",
                    max: "10.0",
                    step: "0.1",
                    value: "{cube_rotation_speed}",
                    oninput: move |event| {
                        if let Ok(speed) = event.value().parse::<f32>() {
                            cube_rotation_speed.peek().set_resource(CubeRotationSpeed(speed));
                        }
                    }
                }
            }
            div {
                id: "footer",
                p { "Bevy framerate: {fps}" }
            }
        }
    }
}
