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
    site::{
        update_model_instances, Affiliation, Change, ChangePlugin, Group, Mobility, ModelMarker,
        ModelProperty, Pose,
    },
    widgets::{prelude::*, Inspect},
    ModelPropertyData,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ComboBox, DragValue, Grid, Ui};
use std::collections::HashMap;

// pub trait MobilityWidget {}

#[derive(Resource)]
pub struct MobilityKinds(
    // pub mobility_map: HashMap<String, Box<dyn MobilityWidget + Send + Sync>>,

    // store a function to show the widget if called
    pub HashMap<String, fn(&mut Mobility, &mut Ui)>,
);

impl FromWorld for MobilityKinds {
    fn from_world(_world: &mut World) -> Self {
        MobilityKinds(HashMap::new())
    }
}

#[derive(Default)]
pub struct InspectMobilityPlugin {}

impl Plugin for InspectMobilityPlugin {
    fn build(&self, app: &mut App) {
        app.world.init_component::<ModelProperty<Mobility>>();
        let component_id = app
            .world
            .components()
            .component_id::<ModelProperty<Mobility>>()
            .unwrap();
        app.add_plugins(ChangePlugin::<ModelProperty<Mobility>>::default())
            .add_systems(PreUpdate, update_model_instances::<Mobility>)
            .init_resource::<ModelPropertyData>()
            .world
            .resource_mut::<ModelPropertyData>()
            .optional
            .insert(
                component_id,
                (
                    "Mobility".to_string(),
                    |mut e_cmd| {
                        e_cmd.insert(ModelProperty::<Mobility>::default());
                    },
                    |mut e_cmd| {
                        e_cmd.remove::<ModelProperty<Mobility>>();
                    },
                ),
            );

        // Ui
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
        (With<ModelMarker>, Without<Group>, With<Mobility>),
    >,
    model_descriptions:
        Query<'w, 's, &'static ModelProperty<Mobility>, (With<ModelMarker>, With<Group>)>,
    change_mobility: EventWriter<'w, Change<ModelProperty<Mobility>>>,
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
        let Ok(ModelProperty(mobility)) = params.model_descriptions.get(description_entity) else {
            return;
        };

        let mut new_mobility = mobility.clone();
        let selected_mobility_kind = if !new_mobility.is_empty() {
            new_mobility.kind.clone()
        } else {
            "Select Kind".to_string()
        };

        ui.label("Mobility");
        ui.indent("inspect_mobility", |ui| {
            Grid::new("inspect_collision_radius")
                .num_columns(3)
                .show(ui, |ui| {
                    ui.label("Collision Radius");
                    if ui
                        .add(
                            DragValue::new(&mut new_mobility.collision_radius)
                                .clamp_range(0_f32..=std::f32::INFINITY)
                                .speed(0.01),
                        )
                        .is_pointer_button_down_on()
                    {
                        if let Ok(pose) = params.poses.get(selection) {
                            params.gizmos.circle(
                                Vec3::new(pose.trans[0], pose.trans[1], pose.trans[2] + 0.01),
                                Vec3::Z,
                                new_mobility.collision_radius,
                                Color::RED,
                            );
                        }
                    };
                    ui.label("m");
                    ui.end_row();

                    ui.label("Center Offset");
                    ui.label("x");
                    ui.label("y");
                    ui.end_row();

                    ui.label("");
                    ui.add(
                        DragValue::new(&mut new_mobility.rotation_center_offset[0])
                            .clamp_range(std::f32::NEG_INFINITY..=std::f32::INFINITY)
                            .speed(0.01),
                    );
                    ui.add(
                        DragValue::new(&mut new_mobility.rotation_center_offset[1])
                            .clamp_range(std::f32::NEG_INFINITY..=std::f32::INFINITY)
                            .speed(0.01),
                    );
                    ui.end_row();
                    ui.label("Bidirectional");
                    ui.checkbox(&mut new_mobility.bidirectional, "");
                });

            ui.add_space(10.0);
            ComboBox::from_id_source("select_mobility_kind")
                .selected_text(selected_mobility_kind)
                .show_ui(ui, |ui| {
                    for (kind, _) in params.mobility.0.iter() {
                        ui.selectable_value(&mut new_mobility.kind, kind.clone(), kind.clone());
                    }
                });
            if !new_mobility.is_default() {
                if let Some(show_widget) = params.mobility.0.get(&new_mobility.kind) {
                    show_widget(&mut new_mobility, ui);
                }
            }
        });

        if new_mobility != *mobility {
            params
                .change_mobility
                .send(Change::new(ModelProperty(new_mobility), description_entity));
        }
        ui.add_space(10.0);
    }
}
