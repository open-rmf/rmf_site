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
    interaction::{Hover, ObjectPlacement},
    site::{FrameMarker, NameInWorkcell, NameOfWorkcell},
    widgets::{prelude::*, Icons, Inspect, SelectorWidget},
    CurrentWorkspace,
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
    icons: Res<'w, Icons>,
    selector: SelectorWidget<'w, 's>,
    object_placement: ObjectPlacement<'w, 's>,
    current_workspace: Res<'w, CurrentWorkspace>,
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
                ui.label("Parent Frame");
                self.selector.show_widget(parent, ui);
                let assign_response = ui.add(ImageButton::new(self.icons.edit.egui()));
                if assign_response.hovered() {
                    self.selector.hover.send(Hover(Some(id)));
                }

                let parent_replace = assign_response.clicked();
                assign_response.on_hover_text("Reassign");

                if parent_replace {
                    if let Some(workspace) = self.current_workspace.root {
                        self.object_placement.replace_parent_3d(id, workspace)
                    } else {
                        warn!("Cannot replace a parent when no workspace is active");
                    }
                }
            });
        }
    }
}
