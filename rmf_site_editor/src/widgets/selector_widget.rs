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

use crate::{
    interaction::{Hover, Select, Selection},
    site::SiteID,
    widgets::{prelude::*, Icons},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, Ui};

/// A widget that can be used to select entities.
#[derive(SystemParam)]
pub struct SelectorWidget<'w, 's> {
    pub site_id: Query<'w, 's, &'static SiteID>,
    pub icons: Res<'w, Icons>,
    pub selection: Res<'w, Selection>,
    pub select: EventWriter<'w, Select>,
    pub hover: EventWriter<'w, Hover>,
}

impl<'w, 's> WidgetSystem<Entity, ()> for SelectorWidget<'w, 's> {
    fn show(entity: Entity, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        params.show_widget(entity, ui);
    }
}

impl<'w, 's> SelectorWidget<'w, 's> {
    pub fn show_widget(&mut self, entity: Entity, ui: &mut Ui) {
        let site_id = self.site_id.get(entity).ok().cloned();
        let is_selected = self.selection.0.is_some_and(|s| s == entity);

        let text = match site_id {
            Some(id) => format!("#{}", id.0),
            None => "*".to_owned(),
        };

        let icon = if is_selected {
            self.icons.selected.egui()
        } else {
            self.icons.select.egui()
        };

        let response = ui.add(Button::image_and_text(icon, text));

        if response.clicked() {
            self.select.send(Select::new(Some(entity)));
        } else if response.hovered() {
            self.hover.send(Hover(Some(entity)));
        }

        response.on_hover_text("Select");
    }
}

impl<'w, 's> ShareableWidget for SelectorWidget<'w, 's> {}
