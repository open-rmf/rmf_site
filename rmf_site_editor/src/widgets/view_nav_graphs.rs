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
    site::{NavGraphMarker, NameInSite, DisplayColor, Change},
    widgets::{
        inspector::color_edit,
        AppEvents,
    },
};
use bevy::{prelude::*, ecs::system::SystemParam};
use bevy_egui::egui::Ui;
use smallvec::SmallVec;

#[derive(SystemParam)]
pub struct NavGraphParams<'w, 's> {
    pub graphs: Query<'w, 's, (Entity, &'static NameInSite, &'static DisplayColor, &'static Visibility), With<NavGraphMarker>>,
}

pub struct ViewNavGraphs<'a, 'w1, 's1, 'w2, 's2> {
    params: &'a NavGraphParams<'w1, 's1>,
    events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> ViewNavGraphs<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(params: &'a NavGraphParams<'w1, 's1>, events: &'a mut AppEvents<'w2, 's2>) -> Self {
        Self { params, events }
    }

    pub fn show(self, ui: &mut Ui) {
        let graphs = {
            let mut graphs: SmallVec<
                [(Entity, &NameInSite, &DisplayColor, &Visibility); 10]
            > = SmallVec::from_iter(self.params.graphs.iter());
            graphs.sort_by(|(a, _, _, _), (b, _, _, _)| a.cmp(b));
            graphs
        };

        for (e, name, color, vis) in graphs {
            ui.horizontal(|ui| {
                let mut is_visible = vis.is_visible;
                if ui.checkbox(&mut is_visible, "")
                    .on_hover_text(if vis.is_visible {
                        "Make this graph invisible"
                    } else {
                        "Make this graph visible"
                    }).changed()
                {
                    self.events.change.visibility.send(Change::new(Visibility { is_visible }, e));
                }

                let mut new_color = color.0;
                color_edit(ui, &mut new_color);
                if new_color != color.0 {
                    self.events.change.color.send(Change::new(DisplayColor(new_color), e));
                }

                let mut new_name = name.0.clone();
                if ui.text_edit_singleline(&mut new_name).changed() {
                    self.events.change.name.send(Change::new(NameInSite(new_name), e));
                }
            });
        }
    }
}
