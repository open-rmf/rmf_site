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

use crate::{recency::RankAdjustment, site::LayerVisibility};
use bevy::{asset::embedded_asset, ecs::system::SystemState, prelude::*};
use bevy_egui::{egui::ImageSource, egui::TextureId, EguiContexts};

/// Add a resource for the common icons of the application.
#[derive(Default)]
pub struct IconsPlugin {}

impl Plugin for IconsPlugin {
    fn build(&self, app: &mut App) {
        add_widgets_icons(app);
        app.init_resource::<Icons>();
    }
}

struct IconBuilder(Handle<Image>);
impl IconBuilder {
    pub fn new(name: &str, asset_server: &AssetServer) -> Self {
        Self(asset_server.load("embedded://librmf_site_editor/".to_owned() + name))
    }

    pub fn build(self, egui_context: &mut EguiContexts) -> Icon {
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
    // TODO(luca) consider exposing parameter for size, all occurences in the codebase currently
    // use [18., 18.]
    pub fn egui(&self) -> ImageSource<'_> {
        ImageSource::Texture((self.egui_handle, [18., 18.].into()).into())
    }
}

/// A collection of icons used by the standard widgets.
#[derive(Clone, Debug, Resource)]
pub struct Icons {
    pub select: Icon,
    pub selected: Icon,
    pub edit: Icon,
    pub exit: Icon,
    pub trash: Icon,
    pub merge: Icon,
    pub confirm: Icon,
    pub add: Icon,
    pub reject: Icon,
    pub search: Icon,
    pub empty: Icon,
    pub alignment: Icon,
    pub layer_up: Icon,
    pub layer_down: Icon,
    pub layer_to_top: Icon,
    pub layer_to_bottom: Icon,
    pub link: Icon,
    pub opaque: Icon,
    pub alpha: Icon,
    pub hidden: Icon,
    pub global: Icon,
    pub hide: Icon,
    pub show: Icon,
    pub home: Icon,
}

impl FromWorld for Icons {
    fn from_world(mut world: &mut World) -> Self {
        let asset_server = world.get_resource::<AssetServer>().unwrap();
        let select = IconBuilder::new("widgets/icons/select.png", &asset_server);
        let selected = IconBuilder::new("widgets/icons/selected.png", &asset_server);
        let edit = IconBuilder::new("widgets/icons/edit.png", &asset_server);
        let exit = IconBuilder::new("widgets/icons/exit.png", &asset_server);
        let trash = IconBuilder::new("widgets/icons/trash.png", &asset_server);
        let merge = IconBuilder::new("widgets/icons/merge.png", &asset_server);
        let confirm = IconBuilder::new("widgets/icons/confirm.png", &asset_server);
        let add = IconBuilder::new("widgets/icons/add.png", &asset_server);
        let reject = IconBuilder::new("widgets/icons/reject.png", &asset_server);
        let search = IconBuilder::new("widgets/icons/search.png", &asset_server);
        let empty = IconBuilder::new("widgets/icons/empty.png", &asset_server);
        let alignment = IconBuilder::new("widgets/icons/alignment.png", &asset_server);
        let layer_up = IconBuilder::new("widgets/icons/up.png", &asset_server);
        let layer_down = IconBuilder::new("widgets/icons/down.png", &asset_server);
        let layer_to_top = IconBuilder::new("widgets/icons/to_top.png", &asset_server);
        let layer_to_bottom = IconBuilder::new("widgets/icons/to_bottom.png", &asset_server);
        let link = IconBuilder::new("widgets/icons/link.png", &asset_server);
        let opaque = IconBuilder::new("widgets/icons/opaque.png", &asset_server);
        let alpha = IconBuilder::new("widgets/icons/alpha.png", &asset_server);
        let hidden = IconBuilder::new("widgets/icons/hidden.png", &asset_server);
        let global = IconBuilder::new("widgets/icons/global.png", &asset_server);
        let hide = IconBuilder::new("widgets/icons/hide.png", &asset_server);
        let show = IconBuilder::new("widgets/icons/show.png", &asset_server);
        let home = IconBuilder::new("widgets/icons/home.png", &asset_server);

        // Note: Building the icons is a two-stage process because we cannot
        // get the mutable EguiContext resource at the same time as the
        // immutable AssetServer resource.
        let mut system_state: SystemState<EguiContexts> = SystemState::new(&mut world);
        let mut egui_context = system_state.get_mut(&mut world);
        Self {
            select: select.build(&mut egui_context),
            selected: selected.build(&mut egui_context),
            edit: edit.build(&mut egui_context),
            exit: exit.build(&mut egui_context),
            trash: trash.build(&mut egui_context),
            merge: merge.build(&mut egui_context),
            confirm: confirm.build(&mut egui_context),
            add: add.build(&mut egui_context),
            reject: reject.build(&mut egui_context),
            search: search.build(&mut egui_context),
            empty: empty.build(&mut egui_context),
            alignment: alignment.build(&mut egui_context),
            layer_up: layer_up.build(&mut egui_context),
            layer_down: layer_down.build(&mut egui_context),
            layer_to_top: layer_to_top.build(&mut egui_context),
            layer_to_bottom: layer_to_bottom.build(&mut egui_context),
            link: link.build(&mut egui_context),
            opaque: opaque.build(&mut egui_context),
            alpha: alpha.build(&mut egui_context),
            hidden: hidden.build(&mut egui_context),
            global: global.build(&mut egui_context),
            hide: hide.build(&mut egui_context),
            show: show.build(&mut egui_context),
            home: home.build(&mut egui_context),
        }
    }
}

impl Icons {
    pub fn layer_visibility_of(&self, vis: Option<LayerVisibility>) -> ImageSource<'_> {
        match vis {
            Some(v) => match v {
                LayerVisibility::Opaque => self.opaque.egui(),
                LayerVisibility::Alpha(_) => self.alpha.egui(),
                LayerVisibility::Hidden => self.hidden.egui(),
            },
            None => self.global.egui(),
        }
    }

    pub fn move_rank(&self, adjustment: RankAdjustment) -> ImageSource<'_> {
        match adjustment {
            RankAdjustment::Delta(delta) => {
                if delta < 0 {
                    self.layer_down.egui()
                } else {
                    self.layer_up.egui()
                }
            }
            RankAdjustment::ToTop => self.layer_to_top.egui(),
            RankAdjustment::ToBottom => self.layer_to_bottom.egui(),
        }
    }
}

fn add_widgets_icons(app: &mut App) {
    embedded_asset!(app, "src/", "icons/add.png");
    embedded_asset!(app, "src/", "icons/alignment.png");
    embedded_asset!(app, "src/", "icons/alpha.png");
    embedded_asset!(app, "src/", "icons/confirm.png");
    embedded_asset!(app, "src/", "icons/down.png");
    embedded_asset!(app, "src/", "icons/edit.png");
    embedded_asset!(app, "src/", "icons/empty.png");
    embedded_asset!(app, "src/", "icons/exit.png");
    embedded_asset!(app, "src/", "icons/global.png");
    embedded_asset!(app, "src/", "icons/hidden.png");
    embedded_asset!(app, "src/", "icons/hide.png");
    embedded_asset!(app, "src/", "icons/home.png");
    embedded_asset!(app, "src/", "icons/link.png");
    embedded_asset!(app, "src/", "icons/merge.png");
    embedded_asset!(app, "src/", "icons/opaque.png");
    embedded_asset!(app, "src/", "icons/reject.png");
    embedded_asset!(app, "src/", "icons/search.png");
    embedded_asset!(app, "src/", "icons/select.png");
    embedded_asset!(app, "src/", "icons/selected.png");
    embedded_asset!(app, "src/", "icons/show.png");
    embedded_asset!(app, "src/", "icons/to_bottom.png");
    embedded_asset!(app, "src/", "icons/to_top.png");
    embedded_asset!(app, "src/", "icons/trash.png");
    embedded_asset!(app, "src/", "icons/up.png");
}
