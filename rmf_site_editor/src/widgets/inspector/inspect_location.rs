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
    site::{
        ConsiderLocationTag, DefaultFile, LocationTag, LocationTags, Model, RecallAssetSource,
        RecallLocationTags, Change,
    },
    widgets::{
        inspector::{InspectAssetSource, InspectName},
        AppEvents, Icons, Inspect,
        prelude::*,
    },
};
use bevy::prelude::*;
use bevy_egui::egui::{ComboBox, ImageButton, RichText, Ui};
use smallvec::SmallVec;

#[derive(SystemParam)]
pub struct InspectLocation<'w, 's> {
    location_tags: Query<'w, 's, (&'static LocationTags, &'static RecallLocationTags)>,
    icons: Res<'w, Icons>,
    consider_tag: EventWriter<'w, ConsiderLocationTag>,
    change_tags: EventWriter<'w, Change<LocationTags>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectLocation<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        params.show_widget(selection, ui);
    }
}

impl<'w, 's> InspectLocation<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let Ok((tags, recall)) = self.location_tags.get(id) else {
            return;
        };

        ui.label(RichText::new("Location Tags").size(18.0));
        let mut deleted_tag = None;
        for (i, tag) in tags.0.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui.add(ImageButton::new(self.icons.trash.egui())).clicked() {
                    deleted_tag = Some(i);
                }
                ui.label(tag.label());
            });
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(5.0);
        }

        let added_tag = ui
            .collapsing("Add...", |ui| {
                let (add, consider) = ui
                    .horizontal(|ui| {
                        let add = ui.button("Confirm").clicked();
                        let mut consider = recall.assume_tag(tags);
                        let mut variants: SmallVec<[LocationTag; 5]> = SmallVec::new();
                        if tags.iter().find(|t| t.is_charger()).is_none() {
                            variants.push(LocationTag::Charger);
                        }
                        if tags.iter().find(|t| t.is_parking_spot()).is_none() {
                            variants.push(LocationTag::ParkingSpot);
                        }
                        if tags.iter().find(|t| t.is_holding_point()).is_none() {
                            variants.push(LocationTag::HoldingPoint);
                        }
                        variants.push(recall.assume_spawn_robot());
                        variants.push(recall.assume_workcell());

                        ComboBox::from_id_source("Add Location Tag")
                            .selected_text(consider.label())
                            .show_ui(ui, |ui| {
                                for variant in variants {
                                    let label = variant.label();
                                    ui.selectable_value(&mut consider, variant, label);
                                }
                            });

                        (add, consider)
                    })
                    .inner;

                let consider_changed = if let Some(original) = &recall.consider_tag {
                    consider != *original
                } else {
                    true
                };
                if consider_changed {
                    self.consider_tag.send(
                        ConsiderLocationTag::new(Some(consider.clone()), id)
                    );
                }

                if add {
                    Some(consider)
                } else {
                    None
                }
            })
            .body_returned
            .flatten();

        if deleted_tag.is_some() || added_tag.is_some() {
            let mut new_tags = tags.clone();
            if let Some(i) = deleted_tag {
                new_tags.remove(i);
            }

            if let Some(new_tag) = added_tag {
                new_tags.push(new_tag);
            }

            self.change_tags.send(Change::new(new_tags, id));
        }
    }
}

pub struct InspectLocationWidget<'a, 'w1, 'w2, 's2> {
    pub entity: Entity,
    pub tags: &'a LocationTags,
    pub recall: &'a RecallLocationTags,
    pub icons: &'a Res<'w1, Icons>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's2> InspectLocationWidget<'a, 'w1, 'w2, 's2> {
    pub fn new(
        entity: Entity,
        tags: &'a LocationTags,
        recall: &'a RecallLocationTags,
        icons: &'a Res<'w1, Icons>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            entity,
            tags,
            recall,
            icons,
            events,
        }
    }

    pub fn show(self, ui: &mut Ui) -> Option<LocationTags> {
        ui.label(RichText::new("Location Tags").size(18.0));
        let mut deleted_tag = None;
        for (i, tag) in self.tags.0.iter().enumerate() {
            ui.horizontal(|ui| {
                if ui.add(ImageButton::new(self.icons.trash.egui())).clicked() {
                    deleted_tag = Some(i);
                }
                ui.label(tag.label());
            });
            ui.add_space(5.0);
            ui.separator();
            ui.add_space(5.0);
        }

        let added_tag = ui
            .collapsing("Add...", |ui| {
                let (add, consider) = ui
                    .horizontal(|ui| {
                        let add = ui.button("Confirm").clicked();
                        let mut consider = self.recall.assume_tag(self.tags);
                        let mut variants: SmallVec<[LocationTag; 5]> = SmallVec::new();
                        if self.tags.iter().find(|t| t.is_charger()).is_none() {
                            variants.push(LocationTag::Charger);
                        }
                        if self.tags.iter().find(|t| t.is_parking_spot()).is_none() {
                            variants.push(LocationTag::ParkingSpot);
                        }
                        if self.tags.iter().find(|t| t.is_holding_point()).is_none() {
                            variants.push(LocationTag::HoldingPoint);
                        }
                        variants.push(self.recall.assume_spawn_robot());
                        variants.push(self.recall.assume_workcell());

                        ComboBox::from_id_source("Add Location Tag")
                            .selected_text(consider.label())
                            .show_ui(ui, |ui| {
                                for variant in variants {
                                    let label = variant.label();
                                    ui.selectable_value(&mut consider, variant, label);
                                }
                            });

                        (add, consider)
                    })
                    .inner;

                let consider_changed = if let Some(original) = &self.recall.consider_tag {
                    consider != *original
                } else {
                    true
                };
                if consider_changed {
                    self.events
                        .request
                        .consider_tag
                        .send(ConsiderLocationTag::new(
                            Some(consider.clone()),
                            self.entity,
                        ));
                }

                if add {
                    Some(consider)
                } else {
                    None
                }
            })
            .body_returned
            .flatten();

        if deleted_tag.is_some() || added_tag.is_some() {
            let mut new_tags = self.tags.clone();
            if let Some(i) = deleted_tag {
                new_tags.remove(i);
            }

            if let Some(new_tag) = added_tag {
                new_tags.push(new_tag);
            }

            Some(new_tags)
        } else {
            None
        }
    }

    #[allow(dead_code)]
    fn inspect_model(
        &self,
        ui: &mut Ui,
        model: &Model,
        recall_asset: &RecallAssetSource,
        default_file: Option<&'a DefaultFile>,
    ) -> Option<Model> {
        let new_name = InspectName::new(&model.name).show(ui);
        let new_source =
            InspectAssetSource::new(&model.source, &recall_asset, default_file).show(ui);

        if new_name.is_some() || new_source.is_some() {
            let mut new_model = model.clone();
            if let Some(new_name) = new_name {
                new_model.name = new_name;
            }
            if let Some(new_source) = new_source {
                new_model.source = new_source;
            }

            Some(new_model)
        } else {
            None
        }
    }
}
