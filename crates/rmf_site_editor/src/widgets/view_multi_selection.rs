/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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
    site::{Delete, NameInSite},
    widgets::{prelude::*, MultiEditPoseWidget},
    Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, CollapsingHeader, ImageButton, ScrollArea, Ui};
use rmf_site_egui::*;
use rmf_site_format::{InstanceMarker, SiteID};
use rmf_site_picking::{Select, Selection};

const INSTANCES_VIEWER_HEIGHT: f32 = 200.0;

/// Add a plugin for viewing and editing a list of all levels
#[derive(Default)]
pub struct ViewMultiSelectionPlugin {}

impl Plugin for ViewMultiSelectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PropertiesTilePlugin::<ViewMultiSelection>::new());
    }
}

#[derive(SystemParam)]
pub struct ViewMultiSelection<'w, 's> {
    icons: Res<'w, Icons>,
    model_instances:
        Query<'w, 's, (Entity, &'static NameInSite, &'static SiteID), With<InstanceMarker>>,
    selection: Res<'w, Selection>,
    delete: EventWriter<'w, Delete>,
    select: EventWriter<'w, Select>,
    multi_edit_pose_widget: MultiEditPoseWidget<'w, 's>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewMultiSelection<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        if params.selection.selected.len() < 2 {
            return;
        }

        CollapsingHeader::new("ViewMultiSelection")
            .default_open(true)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewMultiSelection<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        ScrollArea::vertical()
            .max_height(INSTANCES_VIEWER_HEIGHT)
            .show(ui, |ui| {
                if self.selection.selected.is_empty() {
                    ui.label("Nothing selected or multiple models selected!");
                    return;
                }

                for instance in self.selection.selected.iter() {
                    let Ok((instance_entity, instance_name, site_id)) =
                        self.model_instances.get(*instance)
                    else {
                        continue;
                    };

                    ui.horizontal(|ui| {
                        // Deselect instance from current selection
                        if ui
                            .add(Button::image_and_text(
                                self.icons.deselect.egui(),
                                format!("#{}", site_id.0),
                            ))
                            .on_hover_text("Deselect instance from current selections")
                            .clicked()
                        {
                            self.select.write(Select::new(Some(instance_entity), true));
                        }
                        // Delete instance from this site (all scenarios)
                        if ui
                            .add(ImageButton::new(self.icons.trash.egui()))
                            .on_hover_text("Remove instance from all scenarios")
                            .clicked()
                        {
                            self.delete.write(Delete::new(instance_entity));
                        }

                        // Name of selected model instance
                        ui.label(format!("{}", instance_name.0));
                    });
                }

                let selected_instances: Vec<Entity> =
                    self.selection.selected.iter().cloned().collect();

                ui.separator();

                self.multi_edit_pose_widget
                    .show_widget(selected_instances, ui);
            });
    }
}
