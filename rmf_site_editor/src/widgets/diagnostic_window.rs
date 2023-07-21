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

use crate::inspector::SelectionWidget;
use crate::site::{IssueKey, SiteID, SiteProperties};
use crate::ValidateCurrentWorkspace;
use crate::{AppEvents, Icons};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::utils::Uuid;
use bevy_egui::egui::{Button, Checkbox, ComboBox, Context, RichText, ScrollArea, Ui, Window};
use std::collections::HashSet;

#[derive(Resource, Debug, Clone)]
pub struct DiagnosticWindowState {
    pub show: bool,
    pub selected: Option<IssueKey<Entity>>,
}

#[derive(SystemParam)]
pub struct DiagnosticParams<'w, 's> {
    pub icons: Res<'w, Icons>,
    pub site_id: Query<'w, 's, &'static SiteID>,
    pub site_properties: Query<'w, 's, &'static mut SiteProperties<Entity>>,
}

impl Default for DiagnosticWindowState {
    fn default() -> Self {
        Self {
            show: true,
            selected: None,
        }
    }
}

pub struct DiagnosticWindow<'a, 'w1, 's1, 'w2, 's2> {
    events: &'a mut AppEvents<'w1, 's1>,
    params: &'a mut DiagnosticParams<'w2, 's2>,
}

impl<'a, 'w1, 's1, 'w2, 's2> DiagnosticWindow<'a, 'w1, 's1, 'w2, 's2> {
    pub fn new(
        events: &'a mut AppEvents<'w1, 's1>,
        params: &'a mut DiagnosticParams<'w2, 's2>,
    ) -> Self {
        Self { events, params }
    }

    pub fn show(self, ctx: &Context) {
        //let state = &mut self.events.top_menu_events.diagnostic_window;
        let mut state = (*self.events.top_menu_events.diagnostic_window).clone();
        let Some(root) = self.events.request.current_workspace.root else {
            return;
        };
        // TODO(luca) remove this once we want this to work for other types of workspaces, such as
        // workcells
        let Ok(mut props) = self.params.site_properties.get_mut(root) else {
            return;
        };
        Window::new("Validate Site")
            .open(&mut state.show)
            .show(ctx, |ui| {
                ui.label("Filters");
                for (uuid, name) in self.events.top_menu_events.issue_dictionary.iter() {
                    let mut show_category = !props.filtered_issue_kinds.contains(uuid);
                    if ui.add(Checkbox::new(&mut show_category, name)).clicked() {
                        match show_category {
                            true => props.filtered_issue_kinds.remove(uuid),
                            false => props.filtered_issue_kinds.insert(*uuid),
                        };
                    }
                }
                // Now show the issues
                ScrollArea::vertical()
                    .max_height(300.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut issue_still_exists = false;
                        for (issue, parent) in &self.events.top_menu_events.issues {
                            if props.filtered_issue_kinds.contains(&issue.key.kind)
                                || props.filtered_issues.contains(&issue.key)
                                || **parent != root
                            {
                                continue;
                            }
                            // TODO(luca) ui to remove single issue suppressions
                            let mut sel = state.selected.as_ref().is_some_and(|k| *k == issue.key);
                            issue_still_exists |= sel;
                            ui.horizontal(|ui| {
                                if ui
                                    .toggle_value(&mut sel, &issue.brief)
                                    .on_hover_text(&issue.hint)
                                    .clicked()
                                {
                                    state.selected = sel.then(|| issue.key.clone());
                                    issue_still_exists = sel;
                                }
                                if ui.add(Button::new("Suppress")).clicked() {
                                    props.filtered_issues.insert(issue.key.clone());
                                    if sel {
                                        state.selected = None;
                                    }
                                }
                            });
                        }
                        if !issue_still_exists {
                            state.selected = None;
                        }
                    });
                ui.add_space(10.0);

                // Spawn widgets for selected issue
                if let Some(sel) = &state.selected {
                    ui.label("Affected entities");
                    for e in &sel.entities {
                        ui.horizontal(|ui| {
                            SelectionWidget::new(
                                *e,
                                self.params.site_id.get(*e).ok().cloned(),
                                self.params.icons.as_ref(),
                                self.events,
                            )
                            .show(ui);
                        });
                    }
                }

                if ui.add(Button::new("Validate")).clicked() {
                    self.events
                        .top_menu_events
                        .validate_event
                        .send(ValidateCurrentWorkspace);
                }
            });
        *self.events.top_menu_events.diagnostic_window = state;
    }
}
