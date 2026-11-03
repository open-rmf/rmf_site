/*
 * Copyright (C) 2024 Open Source Robotics Foundation
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

use bevy::prelude::*;
use bevy_egui::egui::{self, Ui, WidgetText};
use egui_dock::{DockArea, DockState, Style, TabViewer};
use std::collections::HashSet;

use crate::AppState;
use rmf_site_egui::{
    PanelConfig, PanelSettings, PanelWidgetInput, TabGroup, Tile, TryShowWidgetWorld,
};

const TAB_BAR_HEIGHT: f32 = 24.0;
const CLOSE_TAB_ACTIVE_COLOR: egui::Color32 = egui::Color32::from_rgb(240, 80, 80);

#[derive(Resource)]
pub struct PropertiesPanelState {
    pub dock_state: DockState<Entity>,
    pub known_tabs: HashSet<Entity>,
}

impl Default for PropertiesPanelState {
    fn default() -> Self {
        Self {
            dock_state: DockState::new(vec![]),
            known_tabs: HashSet::new(),
        }
    }
}

pub struct PropertiesTabViewer<'a> {
    pub world: &'a mut World,
    pub settings: PanelSettings,
}

impl<'a> TabViewer for PropertiesTabViewer<'a> {
    type Tab = Entity;

    fn id(&mut self, tab: &mut Self::Tab) -> egui::Id {
        egui::Id::new(*tab)
    }

    fn title(&mut self, tab: &mut Self::Tab) -> WidgetText {
        if let Some(name) = self.world.get::<Name>(*tab) {
            name.as_str().into()
        } else {
            bevy::log::warn!(
                "Properties panel tab entity {:?} is missing a Name component",
                tab
            );
            format!("Tab {:?}", tab).into()
        }
    }

    fn closeable(&mut self, _tab: &mut Self::Tab) -> bool {
        true
    }

    fn ui(&mut self, ui: &mut Ui, tab: &mut Self::Tab) {
        let _ = self.world.try_show_in(
            *tab,
            Tile {
                id: *tab,
                panel: self.settings,
            },
            ui,
        );
    }
}

pub fn show_properties_panel(
    In(PanelWidgetInput { id, context }): In<PanelWidgetInput>,
    world: &mut World,
) {
    let in_display_mode = world
        .get_resource::<State<AppState>>()
        .is_some_and(|s| match s.get() {
            AppState::MainMenu => false,
            AppState::SiteEditor | AppState::SiteVisualizer | AppState::SiteDrawingEditor => true,
        });

    if !in_display_mode {
        return;
    }

    if !world.contains_resource::<PropertiesPanelState>() {
        world.init_resource::<PropertiesPanelState>();
    }

    let tabs: Vec<Entity> = world
        .get::<Children>(id)
        .map(|c| c.to_vec())
        .unwrap_or_default();

    world.resource_scope::<PropertiesPanelState, ()>(|world, mut state| {
        let is_initial_load = state.known_tabs.is_empty();

        let mut new_tabs = Vec::new();
        for tab in &tabs {
            if state.known_tabs.insert(*tab) {
                new_tabs.push(*tab);
            }
        }

        if !new_tabs.is_empty() {
            if is_initial_load {
                let mut top_tabs = Vec::new();
                let mut bottom_tabs = Vec::new();

                for tab in &new_tabs {
                    let group = world.get::<TabGroup>(*tab).copied().unwrap_or_default();
                    match group {
                        TabGroup::Top => top_tabs.push(*tab),
                        TabGroup::Bottom => bottom_tabs.push(*tab),
                    }
                }

                if top_tabs.is_empty() {
                    state.dock_state = DockState::new(bottom_tabs);
                } else {
                    state.dock_state = DockState::new(top_tabs);
                    if !bottom_tabs.is_empty() {
                        state.dock_state.main_surface_mut().split_below(
                            egui_dock::NodeIndex::root(),
                            0.5,
                            bottom_tabs,
                        );
                    }
                }
            } else {
                for tab in new_tabs {
                    state
                        .dock_state
                        .main_surface_mut()
                        .push_to_focused_leaf(tab);
                }
            }
        }

        let tabs_to_remove: Vec<Entity> = state
            .dock_state
            .iter_all_tabs()
            .map(|(_, t)| *t)
            .filter(|t| !tabs.contains(t))
            .collect();

        for tab in tabs_to_remove {
            state.known_tabs.remove(&tab);
            if let Some(index) = state.dock_state.find_tab(&tab) {
                state.dock_state.remove_tab(index);
            }
        }

        let mut style = Style::from_egui(context.style().as_ref());
        style.tab_bar.show_scroll_bar_on_overflow = true;
        style.tab_bar.fill_tab_bar = false;
        style.tab_bar.height = TAB_BAR_HEIGHT;
        style.buttons.close_tab_active_color = CLOSE_TAB_ACTIVE_COLOR;

        let config = world.get::<PanelConfig>(id).copied().unwrap_or_default();
        egui::SidePanel::right("properties_panel")
            .resizable(config.resizable)
            .default_width(config.default_dimension)
            .min_width(200.0)
            .max_width(800.0)
            .show(&context, |ui| {
                ui.horizontal(|ui| {
                    egui::ComboBox::from_id_salt("tabs_manager_menu")
                        .selected_text("Tabs")
                        .width(0.0)
                        .height(400.0)
                        .show_ui(ui, |ui| {
                            for tab in &tabs {
                                let is_open = state.dock_state.find_tab(tab).is_some();
                                let title =
                                    world.get::<Name>(*tab).map(|n| n.as_str()).unwrap_or("Tab");
                                if ui.selectable_label(is_open, title).clicked() {
                                    if is_open {
                                        if let Some(index) = state.dock_state.find_tab(tab) {
                                            state.dock_state.remove_tab(index);
                                        }
                                    } else {
                                        state
                                            .dock_state
                                            .main_surface_mut()
                                            .push_to_focused_leaf(*tab);
                                    }
                                }
                            }

                            ui.separator();
                            if ui.button("Restore All Tabs").clicked() {
                                for tab in &tabs {
                                    if state.dock_state.find_tab(tab).is_none() {
                                        state
                                            .dock_state
                                            .main_surface_mut()
                                            .push_to_focused_leaf(*tab);
                                    }
                                }
                            }
                        });

                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.label(egui::RichText::new("Properties").weak());
                    });
                });

                ui.separator();

                let settings = world
                    .get::<PanelSettings>(id)
                    .copied()
                    .unwrap_or(PanelSettings::right());
                let mut tab_viewer = PropertiesTabViewer { world, settings };
                DockArea::new(&mut state.dock_state)
                    .style(style)
                    .show_close_buttons(true)
                    .draggable_tabs(true)
                    .show_inside(ui, &mut tab_viewer);
            });
    });
}
