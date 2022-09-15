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

pub mod inspect_anchor_dependency;
pub use inspect_anchor_dependency::*;

pub mod inspect_lane;
pub use inspect_lane::*;

use crate::{
    site::SiteID,
    interaction::Selection,
};
use rmf_site_format::{
    Lane,
};
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::{
    egui::{Widget, Label},
};

#[derive(SystemParam)]
pub struct InspectorParams<'w, 's> {
    pub selection: Res<'w, Selection>,
    pub site_id: Query<'w, 's, Option<&'static SiteID>>,
    pub anchor_params: InspectAnchorParams<'w, 's>,
    pub anchor_dependency_params: InspectAnchorDependencyParams<'w, 's>,
    pub lanes: Query<'w, 's, &'static Lane<Entity>>,

}

pub struct InspectorWidget<'a, 'w, 's> {
    pub params: &'a mut InspectorParams<'w, 's>,
}

impl<'a, 'w, 's> InspectorWidget<'a, 'w, 's> {
    pub fn show(self, ui: &mut bevy_egui::egui::Ui) {
        if let Some(selection) =  self.params.selection.0 {
            let site_id = self.params.site_id.get(selection).ok().flatten();
            if self.params.anchor_params.anchors.contains(selection) {
                ui.add(InspectAnchorWidget::new(
                    selection, &mut self.params.anchor_params,
                ));
            } else if let Ok(lane) = self.params.lanes.get(selection) {
                ui.add(InspectLaneWidget::new(
                    lane, site_id, &mut self.params.anchor_dependency_params,
                ));
            } else {
                ui.add(
                    Label::new("Unsupported selection type")
                    .wrap(false)
                );
            }
        } else {
            ui.add(
                Label::new("Nothing selected")
                .wrap(false)
            );
        }
    }
}
