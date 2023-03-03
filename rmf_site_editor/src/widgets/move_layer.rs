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
    widgets::{RankEvents, Icons},
    recency::{ChangeRank, RankAdjustment},
};
use bevy::prelude::*;
use bevy_egui::egui::{ImageButton, Ui};

pub struct MoveLayer<'a, 'w, 's, T: Component> {
    entity: Entity,
    rank_events: &'a mut EventWriter<'w, 's, ChangeRank<T>>,
    icons: &'a Icons,
    up: bool,
}

impl<'a, 'w, 's, T: Component> MoveLayer<'a, 'w, 's, T> {
    pub fn up(
        entity: Entity,
        rank_events: &'a mut EventWriter<'w, 's, ChangeRank<T>>,
        icons: &'a Icons,
    ) -> Self {
        Self { entity, rank_events, icons, up: true }
    }

    pub fn down(
        entity: Entity,
        rank_events: &'a mut EventWriter<'w, 's, ChangeRank<T>>,
        icons: &'a Icons,
    ) -> Self {
        Self { entity, rank_events, icons, up: false }
    }

    pub fn show(self, ui: &mut Ui) {
        let (icon, text, delta) = if self.up {
            (self.icons.egui_layer_up, "Move up a layer", RankAdjustment::Delta(1))
        } else {
            (self.icons.egui_layer_down, "Move down a layer", RankAdjustment::Delta(-1))
        };

        if ui
            .add(ImageButton::new(icon, [18., 18.]))
            .on_hover_text(text)
            .clicked() {
            self.rank_events.send(ChangeRank::new(self.entity, delta));
        }
    }
}
