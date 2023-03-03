/*
 * Copyright (C) 2022 Open Source Robotics Foundation
 *
 * Licensed under the Apache License, Version 2.0 (the "License");
 * you may not use this file except in compliance with the License.
 * You may obtain a copy of the License at
 *
 *     http://www.apache.org/licenses/LICENSE-2.0
 *
 * Unless required by applicable law or agreed to in writing, software
 * distributed under the License is distributed on an "AS IS" BASIS,
 * WITHOUT WARRANTIES OR CONDITIONS OF ANY KIND, either express or implied.
 * See the License for the specific language governing permissions and
 * limitations under the License.
 *
*/

use crate::site::FloorVisibility;
use bevy::prelude::*;
use bevy_egui::{egui::TextureId, EguiContext};
use rmf_site_format::AssetSource;

struct IconBuilder(Handle<Image>);
impl IconBuilder {
    pub fn new(
        name: &str,
        asset_server: &AssetServer,
    ) -> Self {
        Self(asset_server.load(
            &String::from(&AssetSource::Remote(name.to_owned()))
        ))
    }

    pub fn build(self, egui_context: &mut EguiContext) -> Icon {
        let egui_handle = egui_context.add_image(self.0.clone());
        Icon {
            bevy_handle: self.0,
            egui_handle,
        }
    }
}

#[derive(Clone, Debug)]
pub struct Icon {
    pub bevy_handle: Handle<Image>,
    pub egui_handle: TextureId,
}

impl Icon {
    pub fn egui(&self) -> TextureId {
        self.egui_handle
    }
}

// TODO(MXG): Create a struct to manage bevy-egui image pairs
#[derive(Clone, Debug, Resource)]
pub struct Icons {
    pub select: Icon,
    pub edit: Icon,
    pub trash: Icon,
    pub layer_up: Icon,
    pub layer_down: Icon,
    pub opaque: Icon,
    pub alpha: Icon,
    pub hidden: Icon,
    pub global: Icon,
}

impl FromWorld for Icons {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let select = IconBuilder::new("textures/select.png", &asset_server);
        let edit = IconBuilder::new("textures/edit.png", &asset_server);
        let trash = IconBuilder::new("textures/trash.png", &asset_server);
        let layer_up = IconBuilder::new("textures/up.png", &asset_server);
        let layer_down = IconBuilder::new("textures/down.png", &asset_server);
        let opaque = IconBuilder::new("textures/opaque.png", &asset_server);
        let alpha = IconBuilder::new("textures/alpha.png", &asset_server);
        let hidden = IconBuilder::new("textures/hidden.png", &asset_server);
        let global = IconBuilder::new("textures/global.png", &asset_server);

        // Note: Building the icons is a two-stage process because we cannot
        // get the mutable EguiContext resource at the same time as the
        // immutable AssetServer resource.
        let mut egui_context = world.get_resource_mut::<EguiContext>().unwrap();
        Self {
            select: select.build(&mut egui_context),
            edit: edit.build(&mut egui_context),
            trash: trash.build(&mut egui_context),
            layer_up: layer_up.build(&mut egui_context),
            layer_down: layer_down.build(&mut egui_context),
            opaque: opaque.build(&mut egui_context),
            alpha: alpha.build(&mut egui_context),
            hidden: hidden.build(&mut egui_context),
            global: global.build(&mut egui_context),
        }
    }
}

impl Icons {
    pub fn floor_visibility_of(&self, vis: Option<FloorVisibility>) -> TextureId {
        match vis {
            Some(v) => {
                match v {
                    FloorVisibility::Opaque => self.opaque.egui(),
                    FloorVisibility::Alpha(_) => self.alpha.egui(),
                    FloorVisibility::Hidden => self.hidden.egui(),
                }
            }
            None => self.global.egui(),
        }
    }
}
