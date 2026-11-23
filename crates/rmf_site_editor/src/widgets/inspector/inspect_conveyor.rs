/*
 * Copyright (C) 2026 Open Source Robotics Foundation
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
        validate_conveyor_port_direction, Anchor, AnchorBundle, Change, ConveyorCapacity,
        ConveyorHeight, ConveyorMetadata, ConveyorNode, ConveyorNodePortConfigs, ConveyorPort,
        ConveyorPortDirection, ConveyorSpeed, ConveyorThickness, ConveyorWidth, Delete, Dependents,
        Subordinate, DEFAULT_CONVEYOR_HEIGHT, DEFAULT_CONVEYOR_WIDTH,
    },
    widgets::{inspector::InspectValue, prelude::*, Icons, Inspect},
};
use bevy::{ecs::hierarchy::ChildOf, prelude::*};
use bevy_egui::egui::{ComboBox, DragValue, Grid, ImageButton, RichText, Ui};
use rmf_site_egui::WidgetSystem;
use rmf_site_format::{Affiliation, ConveyorMarker, Edge, Group, ModelMarker, Scale};

#[derive(SystemParam)]
pub struct InspectConveyor<'w, 's> {
    commands: Commands<'w, 's>,
    conveyors: Query<
        'w,
        's,
        (
            &'static Edge<Entity>,
            &'static ConveyorHeight,
            &'static ConveyorWidth,
            &'static ConveyorThickness,
            &'static ConveyorMetadata,
        ),
        With<ConveyorMarker>,
    >,
    models: Query<
        'w,
        's,
        (
            Option<&'static Children>,
            &'static Scale,
            &'static Affiliation<Entity>,
            &'static GlobalTransform,
        ),
        (With<ModelMarker>, Without<Group>, With<ConveyorNode>),
    >,
    ports: Query<'w, 's, (&'static ConveyorPort, &'static ChildOf)>,
    dependents: Query<'w, 's, &'static Dependents, With<Anchor>>,
    icons: Res<'w, Icons>,
    delete: EventWriter<'w, Delete>,
    new_port_direction: Local<'s, ConveyorPortDirection>,
}

fn show_port_direction(ui: &mut Ui, direction: &mut ConveyorPortDirection) {
    ComboBox::from_id_salt("conveyor_port_direction")
        .selected_text(direction.label())
        .width(0.0)
        .show_ui(ui, |ui| {
            for variant in [ConveyorPortDirection::In, ConveyorPortDirection::Out] {
                ui.selectable_value(direction, variant, variant.label());
            }
        });
}

fn show_port(
    ui: &mut Ui,
    entity: Entity,
    port: &ConveyorPort,
    model_scale: Vec3,
    with_labels: bool,
) -> Option<ConveyorPort> {
    let mut new_port = port.clone();
    let scaled_pos = Vec3::from_array(port.location) * model_scale;
    let mut pos = scaled_pos;
    let old_width = port.width.unwrap_or(DEFAULT_CONVEYOR_WIDTH);
    let mut width = old_width;

    let show_field = |ui: &mut Ui, label: &str, add_widget: &mut dyn FnMut(&mut Ui)| {
        if with_labels {
            ui.label(label);
        }
        add_widget(ui);
        if with_labels {
            ui.end_row();
        }
    };

    show_field(ui, "Direction", &mut |ui| {
        ui.push_id(entity, |ui| {
            show_port_direction(ui, &mut new_port.direction);
        });
    });
    for (label, val) in [("x", &mut pos.x), ("y", &mut pos.y), ("z", &mut pos.z)] {
        show_field(ui, label, &mut |ui| {
            ui.add(DragValue::new(val).speed(0.01));
        });
    }
    show_field(ui, "width", &mut |ui| {
        ui.add(
            DragValue::new(&mut width)
                .range(0.001..=std::f32::INFINITY)
                .speed(0.01),
        );
    });

    if pos != scaled_pos {
        new_port.location = (pos / model_scale).to_array();
    }
    if width != old_width {
        new_port.width = Some(width);
    }
    (&new_port != port).then_some(new_port)
}

impl<'w, 's> WidgetSystem<Inspect> for InspectConveyor<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let validate_port = |entity: Entity, old_port: &ConveyorPort, new_port: &ConveyorPort| {
            if new_port.direction != old_port.direction {
                if let Err(err) = validate_conveyor_port_direction(
                    entity,
                    new_port.direction,
                    &params.conveyors,
                    &params.ports,
                    &params.dependents,
                ) {
                    error!("{err}");
                    return false;
                }
            }
            true
        };

        if let Ok((children, scale, affiliation, model_global_tf)) = params.models.get(selection) {
            ui.separator();
            ui.label(RichText::new("Conveyor Ports").size(18.0));

            if children.is_some_and(|c| c.iter().any(|e| params.ports.contains(e))) {
                Grid::new("inspect_conveyor_node_ports")
                    .striped(true)
                    .show(ui, |ui| {
                        ui.label("Direction");
                        ui.label("x");
                        ui.label("y");
                        ui.label("z");
                        ui.label("width");
                        ui.label("");
                        ui.end_row();

                        for &child in children.into_iter().flatten() {
                            let Ok((port, _)) = params.ports.get(child) else {
                                continue;
                            };
                            let changed_port = show_port(ui, child, port, scale.0, false);
                            if ui
                                .add(ImageButton::new(params.icons.trash.egui()))
                                .on_hover_text("Remove this port")
                                .clicked()
                            {
                                params.delete.write(Delete::new(child).and_dependents());
                            }
                            ui.end_row();

                            if let Some(new_port) =
                                changed_port.filter(|new_port| validate_port(child, port, new_port))
                            {
                                params.commands.trigger(Change::new(new_port, child));
                            }
                        }
                    });
            }

            ui.add_space(5.0);
            let add_port = ui
                .horizontal(|ui| {
                    let add = ui.button("Add").clicked();
                    show_port_direction(ui, &mut params.new_port_direction);
                    add
                })
                .inner;

            if add_port {
                params
                    .commands
                    .spawn(
                        AnchorBundle::new(Anchor::Translate2D([0.0, 0.0]))
                            .parent_transform(model_global_tf),
                    )
                    .insert((
                        Subordinate(Some(selection)),
                        ConveyorPort {
                            direction: *params.new_port_direction,
                            location: (Vec3::new(0.0, 0.0, DEFAULT_CONVEYOR_HEIGHT) / scale.0)
                                .to_array(),
                            width: Some(DEFAULT_CONVEYOR_WIDTH),
                        },
                        ChildOf(selection),
                    ));
            }

            // Save the current port configurations to the model description to apply across all instances
            ui.add_space(5.0);
            if ui.button("Apply to all").clicked() {
                if let Some(desc_entity) = affiliation.0 {
                    let configs: Vec<ConveyorPort> = children
                        .into_iter()
                        .flatten()
                        .filter_map(|&child| {
                            params.ports.get(child).ok().map(|(port, _)| port.clone())
                        })
                        .collect();
                    params
                        .commands
                        .entity(desc_entity)
                        .insert(ConveyorNodePortConfigs(Some(configs)));
                }
            }
            ui.add_space(10.0);
            return;
        }

        // Inspect a single selected conveyor port anchor
        if let Ok((port, child_of)) = params.ports.get(selection) {
            let Ok((_, scale, _, _)) = params.models.get(child_of.parent()) else {
                return;
            };

            ui.separator();
            ui.label(RichText::new("Conveyor Port").size(18.0));
            Grid::new("inspect_single_conveyor_port").show(ui, |ui| {
                if let Some(new_port) = show_port(ui, selection, port, scale.0, true)
                    .filter(|new_port| validate_port(selection, port, new_port))
                {
                    params.commands.trigger(Change::new(new_port, selection));
                }
            });
            ui.add_space(10.0);
            return;
        }

        let Ok((_, height, width, thickness, metadata)) = params.conveyors.get(selection) else {
            return;
        };

        ui.separator();
        ui.label(RichText::new("Dimensions").size(14.0));
        if let Some(new_height) = InspectValue::<f32>::new("Height", height.0)
            .clamp_range(0.001..=std::f32::INFINITY)
            .speed(0.01)
            .show(ui)
        {
            params
                .commands
                .trigger(Change::new(ConveyorHeight(new_height), selection));
        }
        if let Some(new_width) = InspectValue::<f32>::new("Width", width.0)
            .clamp_range(0.001..=std::f32::INFINITY)
            .speed(0.01)
            .show(ui)
        {
            params
                .commands
                .trigger(Change::new(ConveyorWidth(new_width), selection));
        }
        if let Some(new_thickness) = InspectValue::<f32>::new("Thickness", thickness.0)
            .clamp_range(0.001..=std::f32::INFINITY)
            .speed(0.01)
            .show(ui)
        {
            params
                .commands
                .trigger(Change::new(ConveyorThickness(new_thickness), selection));
        }

        ui.separator();
        ui.label(RichText::new("Metadata").size(14.0));
        let mut new_metadata = *metadata;
        if let Some(new_speed) = InspectValue::<f32>::new("Speed", metadata.speed.0)
            .clamp_range(0.0..=std::f32::INFINITY)
            .speed(0.01)
            .suffix(" m/s")
            .show(ui)
        {
            new_metadata.speed = ConveyorSpeed(new_speed);
        }
        if let Some(new_capacity) =
            InspectValue::<u32>::new("Capacity", metadata.capacity.0).show(ui)
        {
            new_metadata.capacity = ConveyorCapacity(new_capacity);
        }
        if new_metadata != *metadata {
            params
                .commands
                .trigger(Change::new(new_metadata, selection));
        }
        ui.add_space(10.0);
    }
}
