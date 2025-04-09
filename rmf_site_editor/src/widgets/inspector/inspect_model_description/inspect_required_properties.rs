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

use super::{get_selected_description_entity, ModelPropertyQuery};
use crate::{
    site::{
        AssetSource, Change, DefaultFile, Group, ModelLoader, ModelMarker, ModelProperty,
        RecallAssetSource, Scale,
    },
    widgets::{prelude::*, Inspect, InspectAssetSourceComponent, InspectScaleComponent},
    CurrentWorkspace,
};
use bevy::{ecs::system::SystemParam, prelude::*};

#[derive(SystemParam)]
pub struct InspectModelScale<'w, 's> {
    model_instances: ModelPropertyQuery<'w, 's, Scale>,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Scale>, (With<ModelMarker>, With<Group>)>,
    change_scale: EventWriter<'w, Change<ModelProperty<Scale>>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelScale<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Some(description_entity) = get_selected_description_entity(
            selection,
            &params.model_instances,
            &params.model_descriptions,
        ) else {
            return;
        };

        let Ok(ModelProperty(scale)) = params.model_descriptions.get(description_entity) else {
            return;
        };
        if let Some(new_scale) = InspectScaleComponent::new(scale).show(ui) {
            params
                .change_scale
                .send(Change::new(ModelProperty(new_scale), description_entity));
        }
    }
}

#[derive(SystemParam)]
pub struct InspectModelAssetSource<'w, 's> {
    model_instances: ModelPropertyQuery<'w, 's, AssetSource>,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<AssetSource>, (With<ModelMarker>, With<Group>)>,
    change_asset_source: EventWriter<'w, Change<ModelProperty<AssetSource>>>,
    current_workspace: Res<'w, CurrentWorkspace>,
    default_file: Query<'w, 's, &'static DefaultFile>,
    model_loader: ModelLoader<'w, 's>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectModelAssetSource<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Some(description_entity) = get_selected_description_entity(
            selection,
            &params.model_instances,
            &params.model_descriptions,
        ) else {
            return;
        };

        let Ok(ModelProperty(source)) = params.model_descriptions.get(description_entity) else {
            return;
        };

        let default_file = params
            .current_workspace
            .root
            .map(|e| params.default_file.get(e).ok())
            .flatten();

        if let Some(new_source) =
            InspectAssetSourceComponent::new(source, &RecallAssetSource::default(), default_file)
                .show(ui)
        {
            // TODO(@xiyuoh) look into removing Change for description asset source updates
            params.change_asset_source.send(Change::new(
                ModelProperty(new_source.clone()),
                description_entity,
            ));
            params
                .model_loader
                .update_description_asset_source(description_entity, new_source);
        }
    }
}
