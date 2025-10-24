/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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
    AppState, CurrentWorkspace, Icons,
    site::{AlignSiteDrawings, BeginEditDrawing, Change, PixelsPerMeter},
    widgets::{Inspect, InspectValue, prelude::*},
};
use bevy::prelude::*;
use bevy_egui::egui::Button;
use rmf_site_egui::WidgetSystem;

#[derive(SystemParam)]
pub struct InspectDrawing<'w, 's> {
    commands: Commands<'w, 's>,
    pixels_per_meter: Query<'w, 's, &'static PixelsPerMeter>,
    current_workspace: Res<'w, CurrentWorkspace>,
    align_site: EventWriter<'w, AlignSiteDrawings>,
    app_state: Res<'w, State<AppState>>,
    icons: Res<'w, Icons>,
    begin_edit_drawing: EventWriter<'w, BeginEditDrawing>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectDrawing<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Ok(ppm) = params.pixels_per_meter.get(selection) else {
            return;
        };

        if *params.app_state.get() == AppState::SiteEditor {
            ui.add_space(10.0);
            if ui
                .add(Button::image_and_text(
                    params.icons.edit.egui(),
                    "Edit Drawing",
                ))
                .clicked()
            {
                params.begin_edit_drawing.write(BeginEditDrawing(selection));
            }
            ui.add_space(10.0);
        }

        if *params.app_state.get() != AppState::SiteDrawingEditor {
            if ui
                .add(Button::image_and_text(
                    params.icons.alignment.egui(),
                    "Align Drawings",
                ))
                .on_hover_text(
                    "Align all drawings in the site based on their fiducials and measurements",
                )
                .clicked()
            {
                if let Some(site) = params.current_workspace.root {
                    params.align_site.write(AlignSiteDrawings(site));
                }
            }
            ui.add_space(10.0);
        }

        if let Some(new_ppm) = InspectValue::<f32>::new("Pixels per meter", ppm.0)
            .clamp_range(0.0001..=std::f32::INFINITY)
            .tooltip("How many image pixels per meter")
            .show(ui)
        {
            params
                .commands
                .trigger(Change::new(PixelsPerMeter(new_ppm), selection));
        }
    }
}
