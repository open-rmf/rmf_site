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

use super::get_selected_description_entity;
use crate::{
    site::{Affiliation, Change, Group, Mobility, ModelMarker, ModelProperty, Pose, Robot},
    widgets::{prelude::*, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ComboBox, Ui};
use std::collections::HashMap;

#[derive(Resource)]
pub struct MobilityKinds(pub HashMap<String, fn(&mut Mobility, &mut Ui)>);

impl FromWorld for MobilityKinds {
    fn from_world(_world: &mut World) -> Self {
        MobilityKinds(HashMap::new())
    }
}

#[derive(Default)]
pub struct InspectMobilityPlugin {}

impl Plugin for InspectMobilityPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<MobilityKinds>()
            .add_plugins(InspectionPlugin::<InspectMobility>::new());
    }
}

#[derive(SystemParam)]
pub struct InspectMobility<'w, 's> {
    mobility: ResMut<'w, MobilityKinds>,
    model_instances: Query<
        'w,
        's,
        &'static Affiliation<Entity>,
        (With<ModelMarker>, Without<Group>, With<Robot>),
    >,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
    poses: Query<'w, 's, &'static Pose>,
    gizmos: Gizmos<'s>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectMobility<'w, 's> {
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
        let Ok(ModelProperty(robot)) = params.model_descriptions.get(description_entity) else {
            return;
        };
        let mobility = robot
            .properties
            .get(&Mobility::label())
            .and_then(|m| serde_json::from_value::<Mobility>(m.clone()).ok());
        let mut is_mobile = mobility.is_some();

        ui.checkbox(&mut is_mobile, "Mobility");

        if !is_mobile {
            let mut new_robot = robot.clone();
            new_robot.properties.remove(&Mobility::label());
            params
                .change_robot_property
                .send(Change::new(ModelProperty(new_robot), description_entity));
            return;
        }

        let mut new_mobility = match mobility {
            Some(ref m) => m.clone(),
            None => Mobility::default(),
        };

        let selected_mobility_kind = if !new_mobility.is_empty() {
            new_mobility.kind.clone()
        } else {
            "Select Kind".to_string()
        };

        ui.indent("configure_mobility", |ui| {
            ui.horizontal(|ui| {
                ui.label("Mobility Kind");
                ComboBox::from_id_source("select_mobility_kind")
                    .selected_text(selected_mobility_kind)
                    .show_ui(ui, |ui| {
                        for (kind, _) in params.mobility.0.iter() {
                            ui.selectable_value(&mut new_mobility.kind, kind.clone(), kind.clone());
                        }
                    });
            });
            if !new_mobility.is_default() {
                if let Some(show_widget) = params.mobility.0.get(&new_mobility.kind) {
                    show_widget(&mut new_mobility, ui);
                }
            }
        });

        if mobility.is_none()
            || mobility.is_some_and(|m| m != new_mobility && !new_mobility.is_empty())
        {
            if let Ok(new_value) = serde_json::to_value(new_mobility) {
                let mut new_robot = robot.clone();
                new_robot.properties.insert(Mobility::label(), new_value);
                params
                    .change_robot_property
                    .send(Change::new(ModelProperty(new_robot), description_entity));
            }
        }
        ui.add_space(10.0);
    }
}
