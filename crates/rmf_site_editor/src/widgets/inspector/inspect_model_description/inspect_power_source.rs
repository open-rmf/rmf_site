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

use super::{
    get_selected_description_entity,
    inspect_robot_properties::{show_robot_property_widget, RobotPropertyWidgetRegistry},
};
use crate::{
    site::{
        robot_properties::serialize_and_change_robot_property_kind, Battery, Change, Group,
        ModelMarker, ModelProperty, ModelPropertyQuery, PowerSource, RecallPowerSource, Robot,
        RobotProperty,
    },
    widgets::{prelude::*, Inspect},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{DragValue, Grid, Ui};
use smallvec::SmallVec;

#[derive(SystemParam)]
pub struct InspectPowerSource<'w, 's> {
    robot_property_widgets: Res<'w, RobotPropertyWidgetRegistry>,
    model_instances: ModelPropertyQuery<'w, 's, Robot>,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Robot>, (With<ModelMarker>, With<Group>)>,
    power_source: Query<'w, 's, &'static PowerSource, (With<ModelMarker>, With<Group>)>,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
    children: Query<'w, 's, &'static Children>,
    recall_power_source:
        Query<'w, 's, &'static RecallPowerSource, (With<ModelMarker>, With<Group>)>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectPowerSource<'w, 's> {
    fn show(
        Inspect {
            selection,
            inspection: _,
            panel,
        }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let params = state.get_mut(world);
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

        let recall_power_source: Option<PowerSource> =
            match params.recall_power_source.get(description_entity) {
                Ok(recall) => Some(PowerSource {
                    kind: recall.kind.clone().unwrap_or_default(),
                    config: recall.config.clone().unwrap_or_default(),
                }),
                Err(_) => None,
            };

        show_robot_property_widget::<PowerSource>(
            ui,
            params.power_source,
            recall_power_source,
            params.change_robot_property,
            robot,
            &params.robot_property_widgets,
            description_entity,
        );

        // Show children widgets
        if let Some(widget_registration) = params.robot_property_widgets.get(&PowerSource::label())
        {
            let children_widgets: Result<SmallVec<[_; 16]>, _> = params
                .children
                .get(widget_registration.property_widget)
                .map(|c| c.iter().collect());
            let Ok(children_widgets) = children_widgets else {
                return;
            };

            for child in children_widgets {
                let inspect = Inspect {
                    selection,
                    inspection: child,
                    panel,
                };
                ui.add_space(10.0);
                let _ = world.try_show_in(child, inspect, ui);
            }
        }
    }
}

#[derive(SystemParam)]
pub struct InspectBattery<'w, 's> {
    model_instances: ModelPropertyQuery<'w, 's, Robot>,
    model_descriptions: Query<
        'w,
        's,
        (&'static ModelProperty<Robot>, &'static Battery),
        (With<ModelMarker>, With<Group>),
    >,
    change_robot_property: EventWriter<'w, Change<ModelProperty<Robot>>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectBattery<'w, 's> {
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
        let Ok((ModelProperty(robot), battery)) =
            params.model_descriptions.get_mut(description_entity)
        else {
            return;
        };

        let mut new_battery = battery.clone();

        ui.indent("inspect_battery_properties", |ui| {
            Grid::new("inspect_battery").num_columns(3).show(ui, |ui| {
                ui.label("Voltage")
                    .on_hover_text("The nominal voltage of the battery in Volts");
                ui.add(
                    DragValue::new(&mut new_battery.voltage)
                        .range(0_f32..=std::f32::INFINITY)
                        .speed(0.01),
                );
                ui.label("V");
                ui.end_row();

                ui.label("Capacity")
                    .on_hover_text("The nominal capacity of the battery in Ampere-hours");
                ui.add(
                    DragValue::new(&mut new_battery.capacity)
                        .range(0_f32..=std::f32::INFINITY)
                        .speed(0.01),
                );
                ui.label("Ahr");
                ui.end_row();

                ui.label("Charging Current")
                    .on_hover_text("The rated current in Amperes for charging the battery");
                ui.add(
                    DragValue::new(&mut new_battery.charging_current)
                        .range(0_f32..=std::f32::INFINITY)
                        .speed(0.01),
                );
                ui.label("A");
                ui.end_row();
            });
        });

        if new_battery != *battery {
            serialize_and_change_robot_property_kind::<PowerSource, Battery>(
                &mut params.change_robot_property,
                new_battery,
                robot,
                description_entity,
            );
        }
    }
}
