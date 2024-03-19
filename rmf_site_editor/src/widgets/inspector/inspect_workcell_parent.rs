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
    interaction::{ChangeMode, Hover, SelectAnchor3D},
    site::{FrameMarker, MeshConstraint, NameInWorkcell, NameOfWorkcell, SiteID},
    widgets::{inspector::SelectionWidget, AppEvents, Icons},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ImageButton, Ui};

#[derive(SystemParam)]
pub struct InspectWorkcellParentParams<'w, 's> {
    pub parents: Query<'w, 's, &'static Parent>,
    pub workcell_elements: Query<
        'w,
        's,
        Entity,
        Or<(
            With<FrameMarker>,
            With<NameInWorkcell>,
            With<NameOfWorkcell>,
        )>,
    >,
    pub mesh_constraints: Query<'w, 's, &'static MeshConstraint<Entity>>,
    pub site_id: Query<'w, 's, &'static SiteID>,
    pub icons: Res<'w, Icons>,
}

pub struct InspectWorkcellParentWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub entity: Entity,
    pub params: &'a InspectWorkcellParentParams<'w1, 's1>,
    pub events: &'a mut AppEvents<'w2, 's2>,
}

impl<'a, 'w1, 'w2, 's1, 's2> InspectWorkcellParentWidget<'a, 'w1, 'w2, 's1, 's2> {
    pub fn new(
        entity: Entity,
        params: &'a InspectWorkcellParentParams<'w1, 's1>,
        events: &'a mut AppEvents<'w2, 's2>,
    ) -> Self {
        Self {
            entity,
            params,
            events,
        }
    }

    pub fn show(self, ui: &mut Ui) {
        // If the parent is a frame it should be reassignable
        if let Ok(parent) = self
            .params
            .parents
            .get(self.entity)
            .and_then(|p| self.params.workcell_elements.get(**p))
        {
            ui.vertical(|ui| {
                if let Ok(c) = self.params.mesh_constraints.get(self.entity) {
                    ui.label("Mesh Parent");
                    SelectionWidget::new(
                        c.entity,
                        self.params.site_id.get(c.entity).ok().cloned(),
                        self.params.icons.as_ref(),
                        self.events,
                    )
                    .show(ui);
                }
                ui.label("Parent Frame");
                SelectionWidget::new(
                    parent,
                    self.params.site_id.get(parent).ok().cloned(),
                    self.params.icons.as_ref(),
                    self.events,
                )
                .show(ui);

                let assign_response = ui.add(ImageButton::new(self.params.icons.edit.egui()));

                if assign_response.hovered() {
                    self.events.request.hover.send(Hover(Some(self.entity)));
                }

                let parent_replace = assign_response.clicked();
                assign_response.on_hover_text("Reassign");

                if parent_replace {
                    let request =
                        SelectAnchor3D::replace_point(self.entity, parent).for_anchor(Some(parent));
                    self.events
                        .request
                        .change_mode
                        .send(ChangeMode::To(request.into()));
                }
            });
        }
    }
}
