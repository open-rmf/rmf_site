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

use bevy::prelude::*;
use bevy_egui::{egui, EguiContext};
use rmf_site_format::AssetSource;

#[derive(Clone, Debug, Resource)]
pub struct Icons {
    pub bevy_select: Handle<Image>,
    pub egui_select: egui::TextureId,
    pub bevy_edit: Handle<Image>,
    pub egui_edit: egui::TextureId,
    pub bevy_trash: Handle<Image>,
    pub egui_trash: egui::TextureId,
}

impl FromWorld for Icons {
    fn from_world(world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let bevy_select = asset_server.load(&String::from(&AssetSource::Bundled(
            "textures/select.png".to_string(),
        )));
        let bevy_edit = asset_server.load(&String::from(&AssetSource::Bundled(
            "textures/edit.png".to_string(),
        )));
        let bevy_trash = asset_server.load(&String::from(&AssetSource::Bundled(
            "textures/trash.png".to_string(),
        )));

        let mut egui_context = world.get_resource_mut::<EguiContext>().unwrap();
        let egui_select = egui_context.add_image(bevy_select.clone());
        let egui_edit = egui_context.add_image(bevy_edit.clone());
        let egui_trash = egui_context.add_image(bevy_trash.clone());

        Self {
            bevy_select,
            egui_select,
            bevy_edit,
            egui_edit,
            bevy_trash,
            egui_trash,
        }
    }
}
