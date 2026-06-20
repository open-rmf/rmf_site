/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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
    site::{Bottom, Change, Height, RecallBottom, RecallHeight, RecallTop, Top},
    widgets::{prelude::*, Inspect},
};
use bevy::prelude::*;
use bevy_egui::egui::{ComboBox, DragValue, Grid};
use rmf_site_egui::WidgetSystem;

#[derive(SystemParam)]
pub struct InspectHeight<'w, 's> {
    commands: Commands<'w, 's>,
    bottom: Query<'w, 's, (&'static Bottom, &'static RecallBottom)>,
    top: Query<'w, 's, (&'static Top, &'static RecallTop)>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectHeight<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);

        Grid::new("inspect_height").show(ui, |ui| {
            if let Ok((top, recall)) = params.top.get(selection) {
                ui.label("Top");
                ui.push_id("InspectHeight_Top", |ui| {
                    if let Some(new_top) = show_height(ui, &*top, &*recall) {
                        params
                            .commands
                            .trigger(Change::new(Top(new_top), selection));
                    }
                });
                ui.end_row();
            }

            if let Ok((bottom, recall)) = params.bottom.get(selection) {
                ui.label("Bottom");
                ui.push_id("InspectHeight_Bottom", |ui| {
                    if let Some(new_bottom) = show_height(ui, &*bottom, &*recall) {
                        params
                            .commands
                            .trigger(Change::new(Bottom(new_bottom), selection));
                    }
                });
                ui.end_row();
            }
        });
    }
}

fn show_height(ui: &mut Ui, height: &Height, recall: &RecallHeight) -> Option<Height> {
    let mut new_height = *height;
    ComboBox::from_id_salt("height_type")
        .selected_text(height.label())
        .show_ui(ui, |ui| {
            ui.selectable_value(&mut new_height, height.assume_ratio(recall), "ratio");
            ui.selectable_value(&mut new_height, height.assume_fixed(recall), "fixed");
        });

    ui.add(DragValue::new(new_height.value_mut()).speed(0.02));

    if new_height != *height {
        return Some(new_height);
    }

    None
}
