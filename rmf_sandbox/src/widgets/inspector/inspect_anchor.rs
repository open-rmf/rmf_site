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

use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::{
    egui::{
        Widget, Label
    },
};
use crate::{
    site::Anchor,
};

#[derive(SystemParam)]
pub struct InspectAnchorParams<'w, 's> {
    pub anchors: Query<'w, 's, Entity, With<Anchor>>,
}

pub struct InspectAnchorWidget<'a, 'w, 's> {
    pub params: &'a mut InspectAnchorParams<'w, 's>,
}

impl<'a, 'w, 's> Widget for InspectAnchorWidget<'a, 'w, 's> {
    fn ui(self, ui: &mut bevy_egui::egui::Ui) -> bevy_egui::egui::Response {
        ui.add(Label::new(format!(
            "Are there any anchors? {:?}", !self.params.anchors.is_empty()
        )))
    }
}
