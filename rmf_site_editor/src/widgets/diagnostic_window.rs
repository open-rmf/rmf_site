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
use crate::{AppEvents, Icons, Issue};
use crate::{IssueDictionary, ValidateWorkspace};
use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy_egui::egui::{Button, Checkbox, Context, Grid, ImageButton, ScrollArea, Window};

#[derive(Resource, Debug, Clone, Default)]
pub struct DiagnosticWindowState {
    pub show: bool,
    pub selected: Option<IssueKey<Entity>>,
}

#[derive(SystemParam)]
pub struct DiagnosticParams<'w, 's> {
    pub icons: Res<'w, Icons>,
    pub site_id: Query<'w, 's, &'static SiteID>,
    pub site_properties: Query<'w, 's, &'static mut SiteProperties<Entity>>,
    pub validate_event: EventWriter<'w, 's, ValidateWorkspace>,
    pub issue_dictionary: Res<'w, IssueDictionary>,
    pub issues: Query<'w, 's, (&'static Issue, &'static Parent)>,
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
                ui.collapsing("Filters", |ui| {
                    for (uuid, name) in self.params.issue_dictionary.iter() {
                        let mut show_category = !props.filtered_issue_kinds.contains(uuid);
                        if ui.add(Checkbox::new(&mut show_category, name)).clicked() {
                            match show_category {
                                true => props.filtered_issue_kinds.remove(uuid),
                                false => props.filtered_issue_kinds.insert(*uuid),
                            };
                        }
                    }
                });

                ui.collapsing("Suppressed issues", |ui| {
                    let mut clear_suppressions = Vec::new();
                    for (idx, issue) in props.filtered_issues.iter().enumerate() {
                        ui.horizontal(|ui| {
                            let issue_type = self
                                .params
                                .issue_dictionary
                                .get(&issue.kind)
                                .cloned()
                                .unwrap_or("Unknown Type".to_owned());
                            ui.label(issue_type);
                            if ui
                                .add(ImageButton::new(self.params.icons.trash.egui(), [16., 16.]))
                                .on_hover_text("Remove this suppression")
                                .clicked()
                            {
                                clear_suppressions.push(issue.clone());
                            }
                        });
                        Grid::new(format!("diagnostic_suppressed_affected_entities_{}", idx)).show(
                            ui,
                            |ui| {
                                for e in &issue.entities {
                                    SelectionWidget::new(
                                        *e,
                                        self.params.site_id.get(*e).ok().cloned(),
                                        self.params.icons.as_ref(),
                                        self.events,
                                    )
                                    .show(ui);
                                }
                            },
                        );
                        ui.add_space(10.0);
                    }
                    for c in clear_suppressions.iter() {
                        props.filtered_issues.remove(c);
                    }
                });

                ui.label("Active issues");
                // Now show the issues
                ScrollArea::vertical()
                    .max_height(300.0)
                    .auto_shrink([false, false])
                    .show(ui, |ui| {
                        let mut issue_still_exists = false;
                        if self.params.issues.is_empty() {
                            ui.label("No issues found");
                        }
                        for (issue, parent) in &self.params.issues {
                            if props.filtered_issue_kinds.contains(&issue.key.kind)
                                || props.filtered_issues.contains(&issue.key)
                                || **parent != root
                            {
                                continue;
                            }
                            let mut sel = state.selected.as_ref().is_some_and(|k| *k == issue.key);
                            issue_still_exists |= sel;
                            ui.horizontal(|ui| {
                                if ui
                                    .add(ImageButton::new(
                                        self.params.icons.hide.egui(),
                                        [16., 16.],
                                    ))
                                    .on_hover_text("Suppress this issue")
                                    .clicked()
                                {
                                    props.filtered_issues.insert(issue.key.clone());
                                    issue_still_exists = false;
                                }
                                if ui
                                    .toggle_value(&mut sel, &issue.brief)
                                    .on_hover_text(&issue.hint)
                                    .clicked()
                                {
                                    state.selected = sel.then(|| issue.key.clone());
                                    issue_still_exists = sel;
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
                    Grid::new("diagnostic_affected_entities").show(ui, |ui| {
                        for e in &sel.entities {
                            SelectionWidget::new(
                                *e,
                                self.params.site_id.get(*e).ok().cloned(),
                                self.params.icons.as_ref(),
                                self.events,
                            )
                            .show(ui);
                        }
                    });
                }

                if ui.add(Button::new("Validate")).clicked() {
                    self.params.validate_event.send(ValidateWorkspace(root));
                }
            });
        *self.events.top_menu_events.diagnostic_window = state;
    }
}
