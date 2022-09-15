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
    site::{Anchor, SiteID},
    widgets::{Icons, AppEvents},
    interaction::{Hover, Select, MoveTo},
};
use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_egui::{
    egui::{
        self, Widget, Label, Ui, DragValue, ImageButton,
    },
};

#[derive(SystemParam)]
pub struct InspectAnchorParams<'w, 's> {
    pub transforms: Query<'w, 's, &'static Transform, With<Anchor>>,
    pub icons: Res<'w, Icons>,
    pub site_id: Query<'w, 's, &'static SiteID>,
}

pub struct InspectAnchorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub anchor: Entity,
    pub params: &'a mut InspectAnchorParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
    pub is_dependency: bool,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectAnchorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        anchor: Entity,
        params: &'a mut InspectAnchorParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self{anchor, params, events, is_dependency: false}
    }

    pub fn as_dependency(self) -> Self {
        Self{
            is_dependency: true,
            ..self
        }
    }

    pub fn show(self, ui: &mut Ui) {
        if self.is_dependency {
            if let Ok(site_id) = self.params.site_id.get(self.anchor) {
                ui.label(format!("#{}", site_id.0));
            } else {
                // The star symbol means the anchor is unsaved and therefore
                // has no ID assigned yet.
                ui.label("*").on_hover_text("Unsaved");
            }

            let select_response = ui.add(
                ImageButton::new(
                    self.params.icons.egui_select,
                    [18., 18.],
                )
            );

            if select_response.clicked() {
                self.events.select.send(Select(Some(self.anchor)));
            } else if select_response.hovered() {
                self.events.hover.send(Hover(Some(self.anchor)));
            }

            select_response.on_hover_text("Select");

            let assign_response = ui.add(
                ImageButton::new(
                    self.params.icons.egui_edit,
                    [18., 18.],
                )
            );

            // TODO(MXG): React to assign being clicked
            assign_response.on_hover_text("Reassign");
        }

        if let Ok(tf) = self.params.transforms.get(self.anchor) {
            if !self.is_dependency {
                ui.label("x");
            }
            let mut x = tf.translation.x;
            ui.add(DragValue::new(&mut x).speed(0.01));
            // TODO(MXG): Make the drag speed a user-defined setting

            if !self.is_dependency {
                ui.label("y");
            }
            let mut y = tf.translation.y;
            ui.add(DragValue::new(&mut y).speed(0.01));

            if x != tf.translation.x || y != tf.translation.y {
                self.events.move_to.send(MoveTo{
                    entity: self.anchor,
                    transform: Transform::from_translation([x, y, 0.0].into()),
                });
            }
        }
    }
}
