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
    recency::RecencyRanking,
    site::{
        count_scenarios_with_inclusion, Affiliation, Change, CurrentScenario, Delete, DisplayColor,
        GetModifier, Inclusion, LaneMarker, LocationTags, Modifier, NameInSite, NavGraph,
        NavGraphMarker, Pending, ScenarioModifiers, SiteID, UpdateModifier,
        DEFAULT_NAV_GRAPH_COLORS,
    },
    widgets::{
        inspector::color_edit, prelude::*, FileMenu, Icons, MenuEvent, MenuItem, MoveLayerButton,
        SelectorWidget, TextMenuItem,
    },
    AppState, ChangeRank, CurrentWorkspace, WorkspaceLoader, WorkspaceSaver,
};
use bevy::{
    ecs::{hierarchy::ChildOf, relationship::AncestorIter, system::SystemParam},
    prelude::*,
};
use bevy_egui::egui::{CollapsingHeader, ImageButton, ScrollArea, TextEdit, Ui, Widget};
use rmf_site_egui::*;
use rmf_site_format::Edge;

/// Add a widget for viewing and editing navigation graphs.
#[derive(Default)]
pub struct ViewNavGraphsPlugin {}

impl Plugin for ViewNavGraphsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavGraphDisplay>().add_plugins(
            PropertiesTilePlugin::<ViewNavGraphs>::new("Navigation").in_group(TabGroup::Top),
        );
    }
}

#[derive(Default)]
pub struct NavGraphIoPlugin {}

impl Plugin for NavGraphIoPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavGraphIoMenu>()
            .add_systems(Update, handle_nav_graph_io_events);
    }
}

#[derive(SystemParam)]
pub struct ViewNavGraphs<'w, 's> {
    ranking: Query<'w, 's, &'static RecencyRanking<NavGraphMarker>>,
    graphs: Query<
        'w,
        's,
        (
            &'static NameInSite,
            &'static DisplayColor,
            &'static Visibility,
        ),
        With<NavGraphMarker>,
    >,
    lanes: Query<
        'w,
        's,
        (Entity, &'static Edge<Entity>, Option<&'static SiteID>),
        (With<LaneMarker>, Without<Pending>),
    >,
    locations: Query<
        'w,
        's,
        (Entity, &'static NameInSite, Option<&'static SiteID>),
        (With<LocationTags>, Without<Pending>),
    >,
    child_of: Query<'w, 's, &'static ChildOf>,
    scenarios: Query<
        'w,
        's,
        (
            Entity,
            &'static ScenarioModifiers<Entity>,
            &'static Affiliation<Entity>,
        ),
    >,
    current_scenario: Res<'w, CurrentScenario>,
    get_modifier: GetModifier<'w, 's, Modifier<Inclusion>>,
    icons: Res<'w, Icons>,
    current_workspace: ResMut<'w, CurrentWorkspace>,
    display_nav_graph: ResMut<'w, NavGraphDisplay>,
    delete: EventWriter<'w, Delete>,
    change_rank: EventWriter<'w, ChangeRank<NavGraphMarker>>,
    selector: SelectorWidget<'w, 's>,
    commands: Commands<'w, 's>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewNavGraphs<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if *params.app_state.get() != AppState::SiteEditor {
            return;
        }
        params.show_widget(ui);
    }
}

impl<'w, 's> ViewNavGraphs<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        let ranking = match self.current_workspace.root {
            Some(c) => match self.ranking.get(c) {
                Ok(r) => r,
                Err(_) => return,
            },
            None => return,
        };
        let graph_count = ranking.len();

        ui.horizontal(|ui| {
            if self.display_nav_graph.removing {
                if ui
                    .button("View")
                    .on_hover_text("Toggle visibility of graphs")
                    .clicked()
                {
                    self.display_nav_graph.removing = false;
                }
                ui.label("Remove");
            } else {
                ui.label("View");
                if ui
                    .button("Remove")
                    .on_hover_text("Choose a graph to remove")
                    .clicked()
                {
                    self.display_nav_graph.removing = true;
                }
            }
        });

        ui.horizontal(|ui| {
            let add = ui.button("Add").clicked();
            if self.display_nav_graph.color.is_none() {
                let next_color_index = graph_count % DEFAULT_NAV_GRAPH_COLORS.len();
                self.display_nav_graph.color = Some(DEFAULT_NAV_GRAPH_COLORS[next_color_index]);
            }
            if let Some(color) = &mut self.display_nav_graph.color {
                color_edit(ui, color);
            }
            TextEdit::singleline(&mut self.display_nav_graph.name)
                .desired_width(ui.available_width())
                .show(ui);
            if add {
                self.commands
                    .spawn((Transform::default(), Visibility::default()))
                    .insert(NavGraph {
                        name: NameInSite(self.display_nav_graph.name.clone()),
                        color: DisplayColor(self.display_nav_graph.color.unwrap().clone()),
                        marker: Default::default(),
                    });
                self.display_nav_graph.color = None;
            }
        });

        let mut selected_graph = None;
        for e in ranking.iter().rev() {
            let e = *e;
            if self
                .selector
                .selection
                .get_single()
                .is_some_and(|sel| sel == e)
            {
                selected_graph = Some(e);
            }
            let (name, color, vis) = match self.graphs.get(e) {
                Ok(g) => g,
                Err(_) => continue,
            };
            ui.horizontal(|ui| {
                if self.display_nav_graph.removing {
                    if ui.add(ImageButton::new(self.icons.trash.egui())).clicked() {
                        self.delete.write(Delete::new(e));
                        self.display_nav_graph.removing = false;
                    }
                } else {
                    let mut is_visible = !matches!(vis, Visibility::Hidden);
                    if ui
                        .checkbox(&mut is_visible, "")
                        .on_hover_text(if is_visible {
                            "Make this graph invisible"
                        } else {
                            "Make this graph visible"
                        })
                        .changed()
                    {
                        let visibility = if is_visible {
                            Visibility::Inherited
                        } else {
                            Visibility::Hidden
                        };
                        self.commands.trigger(Change::new(visibility, e));
                    }
                }

                self.selector.show_widget(e, ui);

                let mut new_color = color.0;
                color_edit(ui, &mut new_color);
                if new_color != color.0 {
                    self.commands
                        .trigger(Change::new(DisplayColor(new_color), e));
                }

                let mut new_name = name.0.clone();
                if TextEdit::singleline(&mut new_name)
                    .desired_width(ui.available_width())
                    .ui(ui)
                    .changed()
                {
                    self.commands.trigger(Change::new(NameInSite(new_name), e));
                }
            });
        }

        if let Some(e) = selected_graph {
            ui.horizontal(|ui| {
                MoveLayerButton::to_top(e, &mut self.change_rank, &self.icons).show(ui);
                MoveLayerButton::up(e, &mut self.change_rank, &self.icons).show(ui);
                MoveLayerButton::down(e, &mut self.change_rank, &self.icons).show(ui);
                MoveLayerButton::to_bottom(e, &mut self.change_rank, &self.icons).show(ui);
            });
        }

        if let Some(current_scenario_entity) = self.current_scenario.0 {
            ui.separator();
            let root = self.current_workspace.root;
            let mut lanes: Vec<_> = self
                .lanes
                .iter()
                .filter(|(e, _, _)| AncestorIter::new(&self.child_of, *e).any(|p| Some(p) == root))
                .collect();
            lanes.sort_by_key(|(e, _, site_id)| (site_id.map(|s| s.0), e.index()));

            let mut locations: Vec<_> = self
                .locations
                .iter()
                .filter(|(e, _, _)| AncestorIter::new(&self.child_of, *e).any(|p| Some(p) == root))
                .collect();
            locations.sort_by_key(|(e, _, site_id)| (site_id.map(|s| s.0), e.index()));

            ScrollArea::vertical()
                .auto_shrink([false, false])
                .show(ui, |ui| {
                    CollapsingHeader::new("Lanes")
                        .default_open(
                            self.selector
                                .selection
                                .get_single()
                                .is_some_and(|e| self.lanes.contains(e)),
                        )
                        .show(ui, |ui| {
                            if lanes.is_empty() {
                                ui.label("No lanes in site.");
                            }
                            for (lane_entity, edge, _) in lanes {
                                let start_label = self
                                    .selector
                                    .site_id
                                    .get(edge.start())
                                    .map(|id| format!("#{}", id.0))
                                    .unwrap_or_else(|_| "*".to_string());
                                let end_label = self
                                    .selector
                                    .site_id
                                    .get(edge.end())
                                    .map(|id| format!("#{}", id.0))
                                    .unwrap_or_else(|_| "*".to_string());
                                let label = format!("{start_label} - {end_label}");
                                let scenario_count = count_scenarios_with_inclusion(
                                    &self.scenarios,
                                    lane_entity,
                                    &self.get_modifier,
                                );
                                show_nav_element_row(
                                    ui,
                                    &mut self.commands,
                                    &label,
                                    "Lane",
                                    lane_entity,
                                    &mut self.selector,
                                    &mut self.delete,
                                    &self.get_modifier,
                                    current_scenario_entity,
                                    scenario_count,
                                    &self.icons,
                                );
                            }
                        });

                    CollapsingHeader::new("Locations")
                        .default_open(
                            self.selector
                                .selection
                                .get_single()
                                .is_some_and(|e| self.locations.contains(e)),
                        )
                        .show(ui, |ui| {
                            if locations.is_empty() {
                                ui.label("No locations in site.");
                            }
                            for (location_entity, name, _) in locations {
                                let label = if name.0.is_empty() {
                                    "<Unnamed>"
                                } else {
                                    name.0.as_str()
                                };
                                let scenario_count = count_scenarios_with_inclusion(
                                    &self.scenarios,
                                    location_entity,
                                    &self.get_modifier,
                                );
                                show_nav_element_row(
                                    ui,
                                    &mut self.commands,
                                    label,
                                    "Location",
                                    location_entity,
                                    &mut self.selector,
                                    &mut self.delete,
                                    &self.get_modifier,
                                    current_scenario_entity,
                                    scenario_count,
                                    &self.icons,
                                );
                            }
                        });
                });
        }
    }
}

fn show_nav_element_row(
    ui: &mut Ui,
    commands: &mut Commands,
    label: &str,
    kind_label: &str,
    element: Entity,
    selector: &mut SelectorWidget,
    delete: &mut EventWriter<Delete>,
    get_modifier: &GetModifier<Modifier<Inclusion>>,
    scenario: Entity,
    scenario_count: i32,
    icons: &Res<Icons>,
) {
    let inclusion_modifier = get_modifier
        .scenarios
        .get(scenario)
        .ok()
        .and_then(|(scenario_modifiers, _)| scenario_modifiers.get(&element))
        .and_then(|e| get_modifier.modifiers.get(*e).ok());
    ui.horizontal(|ui| {
        selector.show_widget(element, ui);
        if let Some(inclusion_modifier) = inclusion_modifier {
            if **inclusion_modifier == Inclusion::Hidden {
                if ui
                    .add(ImageButton::new(icons.hide.egui()))
                    .on_hover_text(format!("{kind_label} is hidden in this scenario"))
                    .clicked()
                {
                    commands.entity(element).insert(Inclusion::Included);
                }
            } else {
                if ui
                    .add(ImageButton::new(icons.show.egui()))
                    .on_hover_text(format!("{kind_label} is included in this scenario"))
                    .clicked()
                {
                    if get_modifier
                        .scenarios
                        .get(scenario)
                        .is_ok_and(|(_, a)| a.0.is_some())
                    {
                        commands.trigger(UpdateModifier::<Inclusion>::reset(scenario, element));
                    } else {
                        commands.entity(element).insert(Inclusion::Hidden);
                    }
                }
            }
        } else {
            if ui
                .add(ImageButton::new(icons.link.egui()))
                .on_hover_text(format!(
                    "{kind_label} inclusion is inherited in this scenario"
                ))
                .clicked()
            {
                commands.entity(element).insert(Inclusion::Hidden);
            }
        }
        if ui
            .add(ImageButton::new(icons.trash.egui()))
            .on_hover_text(format!(
                "Remove {} from all scenarios",
                kind_label.to_lowercase()
            ))
            .clicked()
        {
            delete.write(Delete::new(element));
        }
        ui.label(label).on_hover_text(format!(
            "{kind_label} is included in {scenario_count} scenarios"
        ));
    });
}

#[derive(Resource)]
pub struct NavGraphDisplay {
    pub color: Option<[f32; 3]>,
    pub name: String,
    pub removing: bool,
}

impl Default for NavGraphDisplay {
    fn default() -> Self {
        Self {
            color: None,
            name: "<Unnamed>".to_string(),
            removing: false,
        }
    }
}

#[derive(Resource)]
pub struct NavGraphIoMenu {
    export_nav_graph: Entity,
    import_nav_graph: Entity,
}

impl NavGraphIoMenu {
    pub fn get_export_widget(&self) -> Entity {
        self.export_nav_graph
    }

    pub fn get_import_widget(&self) -> Entity {
        self.import_nav_graph
    }
}

impl FromWorld for NavGraphIoMenu {
    fn from_world(world: &mut World) -> Self {
        let file_header = world.resource::<FileMenu>().get();
        let export_nav_graph = world
            .spawn((
                MenuItem::Text(TextMenuItem::new("Export Nav Graphs")),
                ChildOf(file_header),
            ))
            .id();

        let import_nav_graph = world
            .spawn((
                MenuItem::Text(TextMenuItem::new("Import Nav Graphs")),
                ChildOf(file_header),
            ))
            .id();

        NavGraphIoMenu {
            export_nav_graph,
            import_nav_graph,
        }
    }
}

fn handle_nav_graph_io_events(
    mut menu_events: EventReader<MenuEvent>,
    nav_graph_menu: Res<NavGraphIoMenu>,
    mut saver: WorkspaceSaver,
    mut loader: WorkspaceLoader,
) {
    for event in menu_events.read() {
        if !event.clicked() {
            continue;
        }

        if event.source() == nav_graph_menu.get_export_widget() {
            saver.export_nav_graphs_to_dialog();
        }

        if event.source() == nav_graph_menu.get_import_widget() {
            loader.import_nav_graphs_from_dialog();
        }
    }
}
