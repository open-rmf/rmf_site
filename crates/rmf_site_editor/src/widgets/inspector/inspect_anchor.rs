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
    interaction::{Hover, MoveTo},
    site::{Anchor, Category, Dependents, Subordinate},
    widgets::{
        inspector::{Inspect, InspectPoseComponent},
        prelude::*,
        Icons, SelectorWidget,
    },
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{DragValue, ImageButton, Ui};
use std::collections::{BTreeMap, BTreeSet};

#[derive(SystemParam)]
pub struct InspectAnchor<'w, 's> {
    anchors: Query<
        'w,
        's,
        (
            &'static Anchor,
            &'static Transform,
            Option<&'static Subordinate>,
        ),
    >,
    icons: Res<'w, Icons>,
    hover: EventWriter<'w, Hover>,
    move_to: EventWriter<'w, MoveTo>,
}

impl<'w, 's> ShareableWidget for InspectAnchor<'w, 's> {}

impl<'w, 's> WidgetSystem<Inspect> for InspectAnchor<'w, 's> {
    fn show(
        Inspect {
            selection: anchor,
            panel,
            ..
        }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        impl_inspect_anchor(
            InspectAnchorInput {
                anchor,
                is_dependency: false,
                panel,
            },
            ui,
            state,
            world,
        );
    }
}

impl<'w, 's> WidgetSystem<InspectAnchorInput, Option<InspectAnchorResponse>>
    for InspectAnchor<'w, 's>
{
    fn show(
        input: InspectAnchorInput,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) -> Option<InspectAnchorResponse> {
        impl_inspect_anchor(input, ui, state, world)
    }
}

pub struct InspectAnchorInput {
    pub anchor: Entity,
    pub is_dependency: bool,
    pub panel: PanelSide,
}

fn impl_inspect_anchor(
    InspectAnchorInput {
        anchor: id,
        is_dependency,
        panel,
    }: InspectAnchorInput,
    ui: &mut Ui,
    state: &mut SystemState<InspectAnchor>,
    world: &mut World,
) -> Option<InspectAnchorResponse> {
    if world.get::<Anchor>(id).is_none() {
        return None;
    }

    let mut replace = false;
    if is_dependency {
        world.show::<SelectorWidget, _, _>(id, ui);

        let mut params = state.get_mut(world);
        let edit_icon = params.icons.edit.egui();
        let assign_response = ui.add(ImageButton::new(edit_icon));

        if assign_response.hovered() {
            params.hover.write(Hover(Some(id)));
        }

        replace = assign_response.clicked();
        assign_response.on_hover_text("Reassign");
    }

    let mut params = state.get_mut(world);

    if let Ok((anchor, tf, subordinate)) = params.anchors.get(id) {
        if let Some(subordinate) = subordinate.map(|s| s.0) {
            panel.orthogonal(ui, |ui| {
                if let Some(boss) = subordinate {
                    ui.label("Subordinate to ").on_hover_text(
                        "The position of a subordinate anchor is \
                        managed by the properties of another entity.",
                    );
                    world.show::<SelectorWidget, _, _>(boss, ui);
                } else {
                    ui.label("Anonymous subordinate");
                }
            });
        } else {
            match anchor {
                Anchor::Translate2D(_) => {
                    if !is_dependency {
                        ui.label("x");
                    }
                    let mut x = tf.translation.x;
                    ui.add(DragValue::new(&mut x).speed(0.01));

                    if !is_dependency {
                        ui.label("y");
                    }
                    let mut y = tf.translation.y;
                    ui.add(DragValue::new(&mut y).speed(0.01));

                    if x != tf.translation.x || y != tf.translation.y {
                        {}
                        params.move_to.write(MoveTo {
                            entity: id,
                            transform: Transform::from_translation([x, y, 0.0].into()),
                        });
                    }
                }
                Anchor::CategorizedTranslate2D(_) => {
                    warn!("Categorized translate inspector not implemented yet");
                }
                Anchor::Pose3D(pose) => {
                    panel.align(ui, |ui| {
                        if let Some(new_pose) = InspectPoseComponent::new(pose).show(ui) {
                            // TODO(luca) Using moveto doesn't allow switching between variants of
                            // Pose3D
                            params.move_to.write(MoveTo {
                                entity: id,
                                transform: new_pose.transform(),
                            });
                        }
                    });
                }
            }
        }
    }

    Some(InspectAnchorResponse { replace })
}

#[derive(SystemParam)]
pub struct InspectAnchorDependents<'w, 's> {
    dependents: Query<'w, 's, &'static Dependents, With<Anchor>>,
    category: Query<'w, 's, &'static Category>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectAnchorDependents<'w, 's> {
    fn show(
        Inspect {
            selection, panel, ..
        }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let params = state.get(world);
        let Ok(dependents) = params.dependents.get(selection) else {
            return;
        };
        if dependents.is_empty() {
            ui.label("No dependents");
            return;
        }

        let mut category_map: BTreeMap<Category, BTreeSet<Entity>> = BTreeMap::new();
        for e in &dependents.0 {
            if let Ok(category) = params.category.get(*e) {
                category_map.entry(*category).or_default().insert(*e);
            } else {
                error!("Broken reference to entity {e:?}");
            }
        }

        panel.align(ui, |ui| {
            ui.heading("Dependencies");
            for (category, entities) in &category_map {
                ui.label(category.label());
                for e in entities {
                    panel.orthogonal(ui, |ui| {
                        world.show::<SelectorWidget, _, _>(*e, ui);
                    });
                }
            }
        });
    }
}

pub struct InspectAnchorResponse {
    pub replace: bool,
}
