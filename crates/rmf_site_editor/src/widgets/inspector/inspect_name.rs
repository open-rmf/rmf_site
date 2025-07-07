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
    site::Change,
    widgets::{prelude::*, Inspect},
};
use bevy::prelude::*;
use bevy_egui::egui::Ui;
use rmf_site_format::NameInSite;
use rmf_site_ui::*;

#[derive(SystemParam)]
pub struct InspectName<'w, 's> {
    names_in_site: Query<'w, 's, &'static NameInSite>,
    change_name_in_site: EventWriter<'w, Change<NameInSite>>,
}

impl<'w, 's> ShareableWidget for InspectName<'w, 's> {}

impl<'w, 's> WidgetSystem<Inspect> for InspectName<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        if let Ok(name) = params.names_in_site.get(selection) {
            let mut new_name = name.clone();
            ui.horizontal(|ui| {
                ui.label("Name");
                ui.text_edit_singleline(&mut new_name.0);
            });
            if new_name != *name {
                params
                    .change_name_in_site
                    .write(Change::new(new_name, selection));
            }
        }
    }
}
