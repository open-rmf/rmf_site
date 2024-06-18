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
    widgets::{
        prelude::*,
        menu_bar::{MenuEvent, MenuItem, MenuVisualizationStates, ToolMenu},
        SelectorWidget,
    },
    site::{Change, FilteredIssueKinds, FilteredIssues, IssueKey},
    Icons, Issue, IssueDictionary, ValidateWorkspace, CurrentWorkspace, AppState,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::{
    EguiContexts,
    egui::{self, Button, Checkbox, Grid, ImageButton, ScrollArea, Ui},
};
use std::collections::HashSet;

#[derive(Default)]
pub struct DiagnosticsPlugin {

}

impl Plugin for DiagnosticsPlugin {
    fn build(&self, app: &mut App) {
        app
            .init_resource::<IssueDictionary>()
            .init_resource::<IssueMenu>()
            .init_resource::<DiagnosticsDisplay>()
            .add_systems(Update, handle_diagnostic_panel_visibility);

        let panel = PanelWidget::new(diagnostics_panel, &mut app.world);
        let widget = Widget::new::<Diagnostics>(&mut app.world);
        app.world.spawn((panel, widget));
    }
}

fn diagnostics_panel(
    In(panel): In<Entity>,
    world: &mut World,
    egui_contexts: &mut SystemState<EguiContexts>,
) {
    if world.resource::<DiagnosticsDisplay>().show {
        let ctx = egui_contexts.get_mut(world).ctx_mut().clone();
        egui::SidePanel::left("diagnsotics")
            .resizable(true)
            .min_width(320.0)
            .show(&ctx, |ui| {
                if let Err(err) = world.try_show(panel, ui) {
                    error!("Unable to display diagnostics panel: {err:?}");
                }
            });
    }
}

#[derive(SystemParam)]
pub struct Diagnostics<'w, 's> {
    icons: Res<'w, Icons>,
    filters: Query<'w, 's, (&'static FilteredIssues<Entity>, &'static FilteredIssueKinds)>,
    issue_dictionary: Res<'w, IssueDictionary>,
    issues: Query<'w, 's, (&'static Issue, &'static Parent)>,
    display_diagnostics: ResMut<'w, DiagnosticsDisplay>,
    current_workspace: ResMut<'w, CurrentWorkspace>,
    validate_workspace: EventWriter<'w, ValidateWorkspace>,
    change_filtered_issues: EventWriter<'w, Change<FilteredIssues<Entity>>>,
    change_filtered_issue_kinds: EventWriter<'w, Change<FilteredIssueKinds>>,
    selector: SelectorWidget<'w, 's>,
}

impl<'w, 's> WidgetSystem for Diagnostics<'w, 's> {
    fn show(_: (), ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        params.show_widget(ui);
    }
}

impl<'w, 's> Diagnostics<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        //let state = &mut self.events.file_events.diagnostic_window;
        let mut state = (*self.display_diagnostics).clone();
        let Some(root) = self.current_workspace.root else {
            return;
        };
        let Ok((filtered_issues, filtered_issue_kinds)) = self.filters.get(root) else {
            return;
        };
        let mut new_filtered_issues = filtered_issues.clone();
        let mut new_filtered_issue_kinds = filtered_issue_kinds.clone();
        ui.vertical(|ui| {
            ui.collapsing("Filters", |ui| {
                for (uuid, name) in self.issue_dictionary.iter() {
                    let mut show_category = !new_filtered_issue_kinds.contains(uuid);
                    if ui.add(Checkbox::new(&mut show_category, name)).clicked() {
                        match show_category {
                            true => new_filtered_issue_kinds.remove(uuid),
                            false => new_filtered_issue_kinds.insert(*uuid),
                        };
                    }
                }
            });

            ui.collapsing("Suppressed issues", |ui| {
                let mut clear_suppressions = Vec::new();
                for (idx, issue) in new_filtered_issues.iter().enumerate() {
                    ui.horizontal(|ui| {
                        let issue_type = self
                            .issue_dictionary
                            .get(&issue.kind)
                            .cloned()
                            .unwrap_or("Unknown Type".to_owned());
                        ui.label(issue_type);
                        if ui
                            .add(ImageButton::new(self.icons.trash.egui()))
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
                                self.selector.show_widget(*e, ui);
                            }
                        },
                    );
                    ui.add_space(10.0);
                }
                for c in clear_suppressions.iter() {
                    new_filtered_issues.remove(c);
                }
            });

            ui.label("Active issues");
            // Now show the issues
            ScrollArea::vertical()
                .max_height(600.0)
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    let mut issue_still_exists = false;
                    if self.issues.is_empty() {
                        ui.label("No issues found");
                    }
                    for (issue, parent) in &self.issues {
                        if new_filtered_issue_kinds.contains(&issue.key.kind)
                            || new_filtered_issues.contains(&issue.key)
                            || **parent != root
                        {
                            continue;
                        }
                        let mut sel = state.selected.as_ref().is_some_and(|k| *k == issue.key);
                        issue_still_exists |= sel;
                        ui.horizontal(|ui| {
                            if ui
                                .add(ImageButton::new(self.icons.hide.egui()))
                                .on_hover_text("Suppress this issue")
                                .clicked()
                            {
                                new_filtered_issues.insert(issue.key.clone());
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
                        self.selector.show_widget(*e, ui);
                    }
                });
            }

            if ui.add(Button::new("Validate")).clicked() {
                self.validate_workspace.send(ValidateWorkspace(root));
            }
            if ui.add(Button::new("Close")).clicked() {
                state.show = false;
            }
        });
        if new_filtered_issues != *filtered_issues {
            self.change_filtered_issues
                .send(Change::new(new_filtered_issues, root));
        }
        if new_filtered_issue_kinds != *filtered_issue_kinds {
            self.change_filtered_issue_kinds
                .send(Change::new(new_filtered_issue_kinds, root));
        }
        *self.display_diagnostics = state;
    }
}

#[derive(Resource, Debug, Clone, Default)]
pub struct DiagnosticsDisplay {
    pub show: bool,
    pub selected: Option<IssueKey<Entity>>,
}

fn handle_diagnostic_panel_visibility(
    mut menu_events: EventReader<MenuEvent>,
    issue_menu: Res<IssueMenu>,
    mut diagnostic_window: ResMut<DiagnosticsDisplay>,
) {
    for event in menu_events.read() {
        if event.clicked() && event.source() == issue_menu.diagnostic_tool {
            diagnostic_window.show = true;
        }
    }
}

#[derive(Resource)]
pub struct IssueMenu {
    diagnostic_tool: Entity,
}

impl FromWorld for IssueMenu {
    fn from_world(world: &mut World) -> Self {
        let target_states = HashSet::from([
            AppState::SiteEditor,
            AppState::SiteDrawingEditor,
            AppState::SiteVisualizer,
        ]);
        // Tools menu
        let diagnostic_tool = world
            .spawn(MenuItem::Text("Diagnostic Tool".to_string()))
            .insert(MenuVisualizationStates(target_states))
            .id();

        let tool_header = world.resource::<ToolMenu>().get();
        world
            .entity_mut(tool_header)
            .push_children(&[diagnostic_tool]);

        IssueMenu { diagnostic_tool }
    }
}
