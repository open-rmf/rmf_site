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
    site::{LocationTags, Model, RecallAssetSource, RecallLocationTags},
    widgets::{
        inspector::{InspectAssetSource, InspectName, InspectOptionString},
        AppEvents, Icons,
    },
};
use bevy::prelude::*;
use bevy_egui::egui::{ComboBox, ImageButton, RichText, Ui};
use smallvec::SmallVec;

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
        let changed_charger =
            InspectOptionString::new("Charger", &self.tags.charger, &self.recall.recall_charger)
                .multiline()
                .default("")
                .show(ui);
        let changed_parking =
            InspectOptionString::new("Parking", &self.tags.parking, &self.recall.recall_parking)
                .multiline()
                .default("")
                .show(ui);
        let changed_holding =
            InspectOptionString::new("Holding", &self.tags.holding, &self.recall.recall_holding)
                .multiline()
                .default("")
                .show(ui);

        if changed_charger.is_some() || changed_parking.is_some() || changed_holding.is_some() {
            return Some(LocationTags {
                charger: changed_charger.unwrap_or_else(|| self.tags.charger.clone()),
                parking: changed_parking.unwrap_or_else(|| self.tags.parking.clone()),
                holding: changed_holding.unwrap_or_else(|| self.tags.holding.clone()),
            });
        }

        return None;
    }

    fn inspect_model(
        &self,
        ui: &mut Ui,
        model: &Model,
        recall_asset: &RecallAssetSource,
    ) -> Option<Model> {
        let new_name = InspectName::new(&model.name).show(ui);
        let new_source = InspectAssetSource::new(&model.source, &recall_asset).show(ui);

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
