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
    widgets::{SelectorWidget, Inspect, Icons, prelude::*},
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{ImageButton, Ui};

#[derive(SystemParam)]
pub struct InspectWorkcellParent<'w, 's> {
    parents: Query<'w, 's, &'static Parent>,
    workcell_elements: Query<
        'w,
        's,
        Entity,
        Or<(
            With<FrameMarker>,
            With<NameInWorkcell>,
            With<NameOfWorkcell>,
        )>,
    >,
    mesh_constraints: Query<'w, 's, &'static MeshConstraint<Entity>>,
    site_id: Query<'w, 's, &'static SiteID>,
    icons: Res<'w, Icons>,
    selector: SelectorWidget<'w, 's>,
    change_mode: ResMut<'w, Events<ChangeMode>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectWorkcellParent<'w, 's> {
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

impl<'w, 's> InspectWorkcellParent<'w, 's> {
    pub fn show_widget(&mut self, id: Entity, ui: &mut Ui) {
        // If the parent is a frame it should be reassignable
        if let Ok(parent) = self
            .parents
            .get(id)
            .and_then(|p| self.workcell_elements.get(**p))
        {
            ui.vertical(|ui| {
                if let Ok(c) = self.mesh_constraints.get(id) {
                    ui.label("Mesh Parent");
                    self.selector.show_widget(c.entity, ui);
                }
                ui.label("Parent Frame");
                self.selector.show_widget(parent, ui);
                let assign_response = ui.add(ImageButton::new(self.icons.edit.egui()));
                if assign_response.hovered() {
                    self.selector.hover.send(Hover(Some(id)));
                }

                let parent_replace = assign_response.clicked();
                assign_response.on_hover_text("Reassign");

                if parent_replace {
                    let request = SelectAnchor3D::replace_point(id, parent)
                        .for_anchor(Some(parent));
                    self.change_mode.send(ChangeMode::To(request.into()));
                }
            });
        }
    }
}
