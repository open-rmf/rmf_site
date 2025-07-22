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
        Change, Delete, DisplayColor, NameInSite, NavGraph, NavGraphMarker,
        DEFAULT_NAV_GRAPH_COLORS,
    },
    widgets::{
        inspector::color_edit, prelude::*, FileMenu, Icons, MenuEvent, MenuItem, MoveLayerButton,
        SelectorWidget, TextMenuItem,
    },
    AppState, ChangeRank, CurrentWorkspace, WorkspaceLoader, WorkspaceSaver,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, ImageButton, Ui};
use rmf_site_egui::*;

/// Add a widget for viewing and editing navigation graphs.
#[derive(Default)]
pub struct ViewNavGraphsPlugin {}

impl Plugin for ViewNavGraphsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<NavGraphDisplay>()
            .add_plugins(PropertiesTilePlugin::<ViewNavGraphs>::new());
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
    icons: Res<'w, Icons>,
    current_workspace: ResMut<'w, CurrentWorkspace>,
    display_nav_graph: ResMut<'w, NavGraphDisplay>,
    delete: EventWriter<'w, Delete>,
    change_visibility: EventWriter<'w, Change<Visibility>>,
    change_name: EventWriter<'w, Change<NameInSite>>,
    change_color: EventWriter<'w, Change<DisplayColor>>,
    change_rank: EventWriter<'w, ChangeRank<NavGraphMarker>>,
    selector: SelectorWidget<'w>,
    commands: Commands<'w, 's>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewNavGraphs<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);
        if *params.app_state.get() != AppState::SiteEditor {
            return;
        }
        CollapsingHeader::new("Navigation")
            .default_open(true)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
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
            ui.text_edit_singleline(&mut self.display_nav_graph.name);
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
            if self.selector.selection.0.is_some_and(|sel| sel == e) {
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
                        self.change_visibility.write(Change::new(visibility, e));
                    }
                }

                self.selector.show_widget(e, ui);

                let mut new_color = color.0;
                color_edit(ui, &mut new_color);
                if new_color != color.0 {
                    self.change_color
                        .write(Change::new(DisplayColor(new_color), e));
                }

                let mut new_name = name.0.clone();
                if ui.text_edit_singleline(&mut new_name).changed() {
                    self.change_name.write(Change::new(NameInSite(new_name), e));
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
    }
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
