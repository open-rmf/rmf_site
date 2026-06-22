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
use bevy_egui::egui::{Button, ImageButton, ScrollArea, Ui};
use rmf_site_egui::WidgetSystem;
use rmf_site_format::{InstanceMarker, SiteID};
use rmf_site_picking::{Hover, Select};

use smallvec::SmallVec;

const INSTANCES_VIEWER_HEIGHT: f32 = 200.0;

#[derive(SystemParam)]
pub struct InspectMultiSelection<'w, 's> {
    icons: Res<'w, Icons>,
    model_instances:
        Query<'w, 's, (Entity, &'static NameInSite, &'static SiteID), With<InstanceMarker>>,
    delete: EventWriter<'w, Delete>,
    select: EventWriter<'w, Select>,
    multi_edit_pose_widget: MultiEditPoseWidget<'w, 's>,
    hover: EventWriter<'w, Hover>,
}

impl<'w, 's> WidgetSystem<SmallVec<[Entity; 16]>, ()> for InspectMultiSelection<'w, 's> {
    fn show(
        instances: SmallVec<[Entity; 16]>,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) -> () {
        let mut params = state.get_mut(world);

        params.show_widget(instances, ui);
    }
}

impl<'w, 's> InspectMultiSelection<'w, 's> {
    pub fn show_widget(&mut self, instances: SmallVec<[Entity; 16]>, ui: &mut Ui) {
        ScrollArea::vertical()
            .max_height(INSTANCES_VIEWER_HEIGHT)
            .show(ui, |ui| {
                for instance in &instances {
                    let Ok((instance_entity, instance_name, site_id)) =
                        self.model_instances.get(*instance)
                    else {
                        continue;
                    };

                    ui.horizontal(|ui| {
                        // Button for deselecting instance from current selection
                        let response = ui
                            .add(Button::image_and_text(
                                self.icons.deselect.egui(),
                                format!("#{}", site_id.0),
                            ))
                            .on_hover_text("Deselect instance from current selections");

                        if response.clicked() {
                            self.select
                                .write(Select::new(Some(instance_entity)).multi_select(true));
                        } else if response.hovered() {
                            self.hover.write(Hover(Some(instance_entity)));
                        }

                        // Button for deleting instance from this site (all scenarios)
                        let response = ui
                            .add(ImageButton::new(self.icons.trash.egui()))
                            .on_hover_text("Remove instance from all scenarios");

                        if response.clicked() {
                            self.delete.write(Delete::new(instance_entity));
                        } else if response.hovered() {
                            self.hover.write(Hover(Some(instance_entity)));
                        }

                        // Name of selected model instance
                        ui.label(format!("{}", instance_name.0));
                    });
                }

                ui.separator();

                self.multi_edit_pose_widget.show_widget(instances, ui);
            });
    }
}
