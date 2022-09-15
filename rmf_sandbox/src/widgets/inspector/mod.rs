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

pub mod inspect_anchor;
pub use inspect_anchor::*;

use crate::interaction::Selection;
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::{
    egui::{Widget, Label},
};

#[derive(SystemParam)]
pub struct InspectorParams<'w, 's> {
    pub anchor_params: InspectAnchorParams<'w, 's>,
    pub selection: Res<'w, Selection>,
}

pub struct InspectorWidget<'a, 'w, 's> {
    pub params: &'a mut InspectorParams<'w, 's>,
}

impl<'a, 'w, 's> Widget for InspectorWidget<'a, 'w, 's> {
    fn ui(self, ui: &mut bevy_egui::egui::Ui) -> bevy_egui::egui::Response {
        if let Some(selection) =  self.params.selection.0 {
            if self.params.anchor_params.anchors.contains(selection) {
                let anchors = InspectAnchorWidget::new(
                    selection, &mut self.params.anchor_params,
                );
                anchors.ui(ui)
            } else {
                ui.add(
                    Label::new("Unsupported selection type")
                    .wrap(false)
                )
            }
        } else {
            ui.add(
                Label::new("Nothing selected")
                .wrap(false)
            )
        }
    }
}
