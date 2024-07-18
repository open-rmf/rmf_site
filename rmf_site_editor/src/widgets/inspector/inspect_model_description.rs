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
    inspector::{InspectAssetSourceComponent, InspectScaleComponent},
    site::{Category, Change, DefaultFile},
    widgets::{prelude::*, Inspect},
    CurrentWorkspace, Icons, WorkspaceMarker,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ComboBox, Ui};
use rmf_site_format::{
    Affiliation, AssetSource, Group, IsStatic, ModelMarker, ModelProperty, NameInSite,
    RecallAssetSource, Scale,
};

#[derive(SystemParam)]
pub struct InspectModelDescription<'w, 's> {
    model_instances: Query<
        'w,
        's,
        (&'static Category, &'static Affiliation<Entity>),
        (With<ModelMarker>, Without<Group>),
    >,
    model_descriptions: Query<
        'w,
        's,
        (
            Entity,
            &'static NameInSite,
            &'static ModelProperty<AssetSource>,
            &'static ModelProperty<Scale>,
        ),
        (With<Group>, With<ModelMarker>),
    >,
    change_affiliation: EventWriter<'w, Change<Affiliation<Entity>>>,
    change_asset_source: EventWriter<'w, Change<ModelProperty<AssetSource>>>,
    change_scale: EventWriter<'w, Change<ModelProperty<Scale>>>,
    current_workspace: Res<'w, CurrentWorkspace>,
    default_file: Query<'w, 's, &'static DefaultFile>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelDescription<'w, 's> {
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

impl<'w, 's> InspectModelDescription<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        let Ok((_, current_description_entity)) = self.model_instances.get(id) else {
            return;
        };
        let Ok((
            current_description_entity,
            current_description_name,
            current_description_source,
            current_description_scale,
        )) = self.model_descriptions.get(
            current_description_entity
                .0
                .expect("Model instances should have valid affiliation"),
        )
        else {
            return;
        };

        ui.separator();
        ui.label("Model Description");

        let mut new_description_entity = current_description_entity.clone();
        ui.horizontal(|ui| {
            ui.label("Description");
            ComboBox::from_id_source("model_description_affiliation")
                .selected_text(current_description_name.0.as_str())
                .show_ui(ui, |ui| {
                    for (entity, name, ..) in self.model_descriptions.iter() {
                        ui.selectable_value(&mut new_description_entity, entity, name.0.as_str());
                    }
                });
        });
        if new_description_entity != current_description_entity {
            self.change_affiliation
                .send(Change::new(Affiliation(Some(new_description_entity)), id));
        }

        // Asset Source
        let default_file = self
            .current_workspace
            .root
            .map(|e| self.default_file.get(e).ok())
            .flatten();
        if let Some(new_source) = InspectAssetSourceComponent::new(
            &current_description_source.0,
            &RecallAssetSource::default(),
            default_file,
        )
        .show(ui)
        {
            self.change_asset_source.send(Change::new(
                ModelProperty(new_source),
                current_description_entity,
            ));
        }

        // Scale
        if let Some(new_scale) = InspectScaleComponent::new(&current_description_scale.0).show(ui) {
            self.change_scale.send(Change::new(
                ModelProperty(new_scale),
                current_description_entity,
            ));
        }
    }
}
