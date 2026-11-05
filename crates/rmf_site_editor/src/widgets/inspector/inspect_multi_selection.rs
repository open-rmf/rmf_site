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
    widgets::{inspector::InspectNavCell, prelude::*, MultiEditPoseWidget},
    Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, ImageButton, RichText, ScrollArea, Ui};
use rmf_site_egui::WidgetSystem;
use rmf_site_format::{GridCellCoord, InstanceMarker, NavCellMarker, SiteID};
use rmf_site_picking::{Hover, Select};

use smallvec::SmallVec;

const INSTANCES_VIEWER_HEIGHT: f32 = 200.0;

#[derive(SystemParam)]
pub struct InspectMultiSelection<'w, 's> {
    icons: Res<'w, Icons>,
    model_instances:
        Query<'w, 's, (Entity, &'static NameInSite, &'static SiteID), With<InstanceMarker>>,
    nav_cells: Query<
        'w,
        's,
        (
            Entity,
            Option<&'static SiteID>,
            &'static GridCellCoord<Entity>,
        ),
        With<NavCellMarker>,
    >,
    inspect_nav_cell: InspectNavCell<'w, 's>,
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
        let selected_nav_cells: Vec<Entity> = instances
            .iter()
            .copied()
            .filter(|&e| self.nav_cells.contains(e))
            .collect();

        if !selected_nav_cells.is_empty() {
            ui.heading(format!("{} NavCells Selected", selected_nav_cells.len()));
            ScrollArea::vertical()
                .id_salt("multi_nav_cells_scroll")
                .max_height(140.0)
                .show(ui, |ui| {
                    for &cell_e in &selected_nav_cells {
                        let Ok((_, site_id, grid_coord)) = self.nav_cells.get(cell_e) else {
                            continue;
                        };
                        let id_label = site_id
                            .map(|s| format!("#{}", s.0))
                            .unwrap_or_else(|| format!("e{}", cell_e.index()));
                        ui.horizontal(|ui| {
                            let response = ui
                                .add(Button::image_and_text(
                                    self.icons.deselect.egui(),
                                    &id_label,
                                ))
                                .on_hover_text("Deselect cell from current selection");
                            if response.clicked() {
                                self.inspect_nav_cell
                                    .select
                                    .write(Select::new(Some(cell_e)).multi_select(true));
                            } else if response.hovered() {
                                self.hover.write(Hover(Some(cell_e)));
                            }

                            if let Some(gc) = grid_coord.0 {
                                ui.label(
                                    RichText::new(format!("Grid (row {}, col {})", gc.row, gc.col))
                                        .small(),
                                );
                            } else {
                                ui.label(RichText::new("Custom Polygon Cell").small());
                            }
                        });
                    }
                });

            self.inspect_nav_cell
                .show_batch(&selected_nav_cells, None, ui);

            if selected_nav_cells.len() == instances.len() {
                return;
            }
            ui.separator();
        }

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
                            self.inspect_nav_cell
                                .select
                                .write(Select::new(Some(instance_entity)).multi_select(true));
                        } else if response.hovered() {
                            self.hover.write(Hover(Some(instance_entity)));
                        }

                        // Button for deleting instance from this site (all scenarios)
                        let response = ui
                            .add(ImageButton::new(self.icons.trash.egui()))
                            .on_hover_text("Remove instance from all scenarios");

                        if response.clicked() {
                            self.inspect_nav_cell
                                .delete
                                .write(Delete::new(instance_entity));
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
