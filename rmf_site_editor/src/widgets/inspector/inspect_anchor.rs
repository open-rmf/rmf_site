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
    interaction::{ChangeMode, Hover, MoveTo, SelectAnchor3D},
    site::{
        latlon_to_world, world_to_latlon, Anchor, AssociatedGraphs, Category, Change, Dependents,
        GeographicComponent, LocationTags, MeshConstraint, SiteID, Subordinate,
    },
    widgets::{inspector::InspectPose, inspector::SelectionWidget, AppEvents, Icons},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{DragValue, ImageButton, Ui};
use std::collections::{BTreeMap, HashSet};

#[derive(SystemParam)]
pub struct InspectAnchorParams<'w, 's> {
    pub anchors: Query<
        'w,
        's,
        (
            &'static Anchor,
            &'static Transform,
            Option<&'static Subordinate>,
            &'static Parent,
            Option<&'static MeshConstraint<Entity>>,
        ),
    >,
    pub icons: Res<'w, Icons>,
    pub site_id: Query<'w, 's, &'static SiteID>,
    pub geographic_offset: Query<'w, 's, &'static GeographicComponent>,
}

pub struct InspectAnchorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub anchor: Entity,
    pub params: &'a InspectAnchorParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
    pub is_dependency: bool,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectAnchorWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        anchor: Entity,
        params: &'a InspectAnchorParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            anchor,
            params,
            events,
            is_dependency: false,
        }
    }

    pub fn as_dependency(self) -> Self {
        Self {
            is_dependency: true,
            ..self
        }
    }

    pub fn show(self, ui: &mut Ui) -> InspectAnchorResponse {
        let mut replace = false;
        if self.is_dependency {
            SelectionWidget::new(
                self.anchor,
                self.params.site_id.get(self.anchor).ok().cloned(),
                self.params.icons.as_ref(),
                self.events,
            )
            .show(ui);

            let assign_response =
                ui.add(ImageButton::new(self.params.icons.edit.egui(), [18., 18.]));

            if assign_response.hovered() {
                self.events.request.hover.send(Hover(Some(self.anchor)));
            }

            replace = assign_response.clicked();
            assign_response.on_hover_text("Reassign");
        }

        if let Ok((anchor, tf, subordinate, parent, mesh_constraint)) =
            self.params.anchors.get(self.anchor)
        {
            if let Some(subordinate) = subordinate {
                ui.horizontal(|ui| {
                    if let Some(boss) = subordinate.0 {
                        ui.label("Subordinate to ").on_hover_text(
                            "The position of a subordinate anchor is \
                                managed by the properties of another entity.",
                        );
                        SelectionWidget::new(
                            boss,
                            self.params.site_id.get(boss).ok().copied(),
                            self.params.icons.as_ref(),
                            self.events,
                        )
                        .show(ui);
                    } else {
                        ui.label("Anonymous subordinate");
                    }
                });
            } else {
                match anchor {
                    Anchor::Translate2D(_anchor) => {
                        ui.vertical(|ui| {
                            ui.horizontal( |ui|  {
                                if !self.is_dependency {
                                    ui.label("x");
                                }
                                let mut x = tf.translation.x;
                                ui.add(DragValue::new(&mut x).speed(0.01));
                                // TODO(MXG): Make the drag speed a user-defined setting

                                if !self.is_dependency {
                                    ui.label("y");
                                }
                                let mut y = tf.translation.y;
                                ui.add(DragValue::new(&mut y).speed(0.01));

                                if x != tf.translation.x || y != tf.translation.y {
                                    self.events.request.move_to.send(MoveTo {
                                        entity: self.anchor,
                                        transform: Transform::from_translation([x, y, 0.0].into()),
                                    });
                                }
                            });

                            for comp in &self.params.geographic_offset {
                                let Some(offset) = comp.0 else {
                                    continue;
                                };
                                let Ok((mut lat, mut  lon))= world_to_latlon(tf.translation, offset.anchor) else {
                                    continue;
                                };

                                let old_lat = lat.clone();
                                let old_lon = lon.clone();

                                if !self.is_dependency {
                                    ui.label("Latitude");
                                    ui.add(DragValue::new(&mut lat).speed(1e-16));
                                    ui.label("Longitude");
                                    ui.add(DragValue::new(&mut lon).speed(1e-16));

                                    if old_lat != lat || old_lon != lon {
                                        self.events.request.move_to.send(MoveTo {
                                            entity: self.anchor,
                                            transform: Transform::from_translation(latlon_to_world(
                                                lat as f32, lon as f32, offset.anchor,
                                            )),
                                        });
                                    }
                                }
                            }
                        });
                    }
                    Anchor::CategorizedTranslate2D(anchor) => {
                        todo!("Categorized translate inspector not implemented yet");
                    }
                    Anchor::Pose3D(pose) => {
                        ui.vertical(|ui| {
                            if let Some(c) = mesh_constraint {
                                // For mesh constraints we only allow rotation and inspection of
                                // parents
                                if let Some(new_pose) =
                                    InspectPose::new(&c.relative_pose).for_rotation().show(ui)
                                {
                                    // TODO(luca) Using moveto doesn't allow switching between variants of
                                    // Pose3D
                                    self.events
                                        .workcell_change
                                        .mesh_constraints
                                        .send(Change::new(
                                            MeshConstraint {
                                                entity: c.entity,
                                                element: c.element.clone(),
                                                relative_pose: new_pose,
                                            },
                                            self.anchor,
                                        ));
                                }
                                ui.label("Mesh Parent");
                                SelectionWidget::new(
                                    c.entity,
                                    self.params.site_id.get(c.entity).ok().cloned(),
                                    self.params.icons.as_ref(),
                                    self.events,
                                )
                                .show(ui);

                                ui.label("Frame Parent");
                                SelectionWidget::new(
                                    parent.get(),
                                    self.params.site_id.get(parent.get()).ok().cloned(),
                                    self.params.icons.as_ref(),
                                    self.events,
                                )
                                .show(ui);
                            } else {
                                if let Some(new_pose) = InspectPose::new(pose).show(ui) {
                                    // TODO(luca) Using moveto doesn't allow switching between variants of
                                    // Pose3D
                                    self.events.request.move_to.send(MoveTo {
                                        entity: self.anchor,
                                        transform: new_pose.transform(),
                                    });
                                }

                                // Parent reassigning widget
                                ui.label("Parent");
                                SelectionWidget::new(
                                    parent.get(),
                                    self.params.site_id.get(parent.get()).ok().cloned(),
                                    self.params.icons.as_ref(),
                                    self.events,
                                )
                                .show(ui);

                                let assign_response = ui.add(ImageButton::new(
                                    self.params.icons.edit.egui(),
                                    [18., 18.],
                                ));

                                if assign_response.hovered() {
                                    self.events.request.hover.send(Hover(Some(self.anchor)));
                                }

                                let parent_replace = assign_response.clicked();
                                assign_response.on_hover_text("Reassign");

                                if parent_replace {
                                    let request =
                                        SelectAnchor3D::replace_point(self.anchor, parent.get())
                                            .for_anchor(Some(parent.get()));
                                    self.events
                                        .request
                                        .change_mode
                                        .send(ChangeMode::To(request.into()));
                                }
                            }
                        });
                    }
                }
            }
        }

        InspectAnchorResponse { replace }
    }
}

pub struct InspectAnchorResponse {
    pub replace: bool,
}

#[derive(SystemParam)]
pub struct InspectAnchorDependentsParams<'w, 's> {
    pub dependents: Query<'w, 's, &'static Dependents, With<Anchor>>,
    pub locations: Query<'w, 's, &'static LocationTags, &'static AssociatedGraphs<Entity>>,
    pub info: Query<'w, 's, (&'static Category, Option<&'static SiteID>)>,
    pub icons: Res<'w, Icons>,
}

pub struct InspectAnchorDependentsWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub anchor: Entity,
    pub params: &'a InspectAnchorDependentsParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectAnchorDependentsWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        anchor: Entity,
        params: &'a InspectAnchorDependentsParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            anchor,
            params,
            events,
        }
    }

    fn show_dependents(
        dependents: &HashSet<Entity>,
        params: &InspectAnchorDependentsParams<'w1, 's1>,
        events: &mut AppEvents<'w2, 's2>,
        ui: &mut Ui,
    ) {
        ui.heading("Dependents");
        let mut category_map: BTreeMap<Category, BTreeMap<Entity, Option<u32>>> = BTreeMap::new();
        for e in dependents {
            if let Ok((category, site_id)) = params.info.get(*e) {
                category_map
                    .entry(*category)
                    .or_default()
                    .insert(*e, site_id.map(|s| s.0));
            } else {
                ui.label(format!("ERROR: Broken reference to entity {e:?}"));
            }
        }

        for (category, entities) in &category_map {
            ui.label(category.label());

            for (e, site_id) in entities {
                ui.horizontal(|ui| {
                    SelectionWidget::new(*e, site_id.map(SiteID), params.icons.as_ref(), events)
                        .show(ui);
                });
            }
        }
    }

    pub fn show(mut self, ui: &mut Ui) {
        ui.vertical(|ui| {
            if let Ok(dependents) = self.params.dependents.get(self.anchor) {
                if dependents.is_empty() {
                    ui.label("No dependents");
                } else {
                    Self::show_dependents(&dependents.0, &self.params, &mut self.events, ui);
                }
            } else {
                ui.label("ERROR: Unable to find dependents info for this anchor");
            }
        });
    }
}
