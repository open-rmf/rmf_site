/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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
    interaction::Hover,
    widgets::Icons,
    recency::{ChangeRank, RankAdjustment},
};
use bevy::prelude::*;
use bevy_egui::egui::{ImageButton, Ui};

pub struct MoveLayer<'a, 'w, 's, T: Component> {
    entity: Entity,
    rank_events: &'a mut EventWriter<'w, 's, ChangeRank<T>>,
    icons: &'a Icons,
    up: bool,
    hover: Option<&'a mut ResMut<'w, Events<Hover>>>,
}

impl<'a, 'w, 's, T: Component> MoveLayer<'a, 'w, 's, T> {
    pub fn up(
        entity: Entity,
        rank_events: &'a mut EventWriter<'w, 's, ChangeRank<T>>,
        icons: &'a Icons,
    ) -> Self {
        Self { entity, rank_events, icons, up: true, hover: None }
    }

    pub fn down(
        entity: Entity,
        rank_events: &'a mut EventWriter<'w, 's, ChangeRank<T>>,
        icons: &'a Icons,
    ) -> Self {
        Self { entity, rank_events, icons, up: false, hover: None }
    }

    pub fn with_hover(mut self, hover: &'a mut ResMut<'w, Events<Hover>>) -> Self {
        self.hover = Some(hover);
        self
    }

    pub fn show(self, ui: &mut Ui) {
        let (icon, text, delta) = if self.up {
            (self.icons.layer_up.egui(), "Move up one layer", RankAdjustment::Delta(1))
        } else {
            (self.icons.layer_down.egui(), "Move down one layer", RankAdjustment::Delta(-1))
        };

        let resp = ui
            .add(ImageButton::new(icon, [18., 18.]))
            .on_hover_text(text);

        if resp.clicked() {
            self.rank_events.send(ChangeRank::new(self.entity, delta));
        }

        if let Some(hover) = self.hover {
            if resp.hovered() {
                hover.send(Hover(Some(self.entity)));
            }
        }
    }
}
