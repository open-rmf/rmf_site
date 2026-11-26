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

use super::demo_world::*;
use crate::{site::LoadSite, AppState, Autoload, WorkspaceLoader};
use bevy::{
    ecs::system::{SystemParam, SystemState},
    prelude::*,
    window::PrimaryWindow,
};
use bevy_egui::{egui, EguiContexts};
use rmf_site_egui::{TryShowWidgetWorld, Widget, WidgetSystem};

pub const MAIN_MENU_PADDING: f32 = 10.0;
const DEFAULT_SECTION_WIDTH: f32 = 350.0;

/// Use this plugin to add a section to the main menu.
pub struct MainMenuSectionPlugin<W>
where
    W: WidgetSystem + 'static + Send + Sync,
{
    _ignore: std::marker::PhantomData<W>,
}

impl<W> MainMenuSectionPlugin<W>
where
    W: WidgetSystem + 'static + Send + Sync,
{
    pub fn new() -> Self {
        Self {
            _ignore: Default::default(),
        }
    }
}

impl<W> Plugin for MainMenuSectionPlugin<W>
where
    W: WidgetSystem + 'static + Send + Sync,
{
    fn build(&self, app: &mut App) {
        let widget = Widget::new::<W>(app.world_mut());
        let main_menu = app.world().resource::<MainMenu>().id;
        app.world_mut().spawn(widget).insert(ChildOf(main_menu));
    }
}

#[derive(Resource)]
pub struct MainMenu {
    id: Entity,
}

impl MainMenu {
    pub fn id(&self) -> Entity {
        self.id
    }
}

#[derive(SystemParam)]
struct MainMenuParams<'w, 's> {
    egui_context: EguiContexts<'w, 's>,
    #[cfg(not(target_arch = "wasm32"))]
    workspace_loader: WorkspaceLoader<'w, 's>,
    autoload: Option<ResMut<'w, Autoload>>,
    primary_windows: Query<'w, 's, Entity, With<PrimaryWindow>>,
}

fn egui_ui(world: &mut World, params_state: &mut SystemState<MainMenuParams>) {
    let mut params = params_state.get_mut(world);
    if let Some(mut autoload) = params.autoload {
        #[cfg(not(target_arch = "wasm32"))]
        {
            if let Some(filename) = autoload.filename.take() {
                let _ = params.workspace_loader.load_from_path(filename);
            }
        }
        params_state.apply(world);
        return;
    }

    let Some(ctx) = params
        .primary_windows
        .single()
        .ok()
        .and_then(|w| params.egui_context.try_ctx_for_entity_mut(w))
        .cloned()
    else {
        return;
    };

    let sections: Vec<Entity> = world
        .get::<Children>(world.resource::<MainMenu>().id)
        .map(|children| children.iter().collect())
        .unwrap_or_default();
    let width = DEFAULT_SECTION_WIDTH * sections.len() as f32;

    egui::Window::new("Welcome!")
        .collapsible(false)
        .resizable(false)
        .title_bar(false)
        .fixed_size(egui::vec2(width, 500.0))
        .anchor(egui::Align2::CENTER_CENTER, egui::vec2(0., 0.))
        .show(&ctx, |ui| {
            ui.add_space(MAIN_MENU_PADDING);
            ui.vertical_centered(|ui| {
                ui.heading("Welcome to The RMF Site Editor!");
            });
            ui.add_space(MAIN_MENU_PADDING);

            ui.columns(sections.len(), |columns| {
                for (column, &section) in columns.iter_mut().zip(&sections) {
                    egui::Frame::NONE
                        .inner_margin(MAIN_MENU_PADDING)
                        .show(column, |ui| {
                            if let Err(err) = world.try_show(section, ui) {
                                error!("Unable to display main menu section: {err:?}");
                            }
                        });
                }

                let top = columns
                    .iter()
                    .map(|column| column.min_rect().top())
                    .fold(f32::INFINITY, f32::min)
                    + MAIN_MENU_PADDING;
                let bottom = columns
                    .iter()
                    .map(|column| column.min_rect().bottom())
                    .fold(f32::NEG_INFINITY, f32::max);
                for pair in columns.windows(2) {
                    let x = (pair[0].max_rect().right() + pair[1].max_rect().left()) * 0.5;
                    let stroke = pair[0].visuals().widgets.noninteractive.bg_stroke;
                    pair[0].painter().vline(x, top..=bottom, stroke);
                }
            });

            #[cfg(not(target_arch = "wasm32"))]
            {
                ui.add_space(MAIN_MENU_PADDING);
                ui.horizontal(|ui| {
                    ui.with_layout(egui::Layout::right_to_left(egui::Align::Center), |ui| {
                        ui.add_space(MAIN_MENU_PADDING);
                        if ui.button("  Exit  ").clicked() {
                            world.send_event(AppExit::Success);
                        }
                    });
                });
                ui.add_space(MAIN_MENU_PADDING);
            }
        });
}

#[derive(SystemParam)]
struct EditorMenuSection<'w, 's> {
    workspace_loader: WorkspaceLoader<'w, 's>,
}

impl<'w, 's> WidgetSystem for EditorMenuSection<'w, 's> {
    fn show(_: (), ui: &mut egui::Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);

        ui.heading("Create a Site:");
        ui.add_space(MAIN_MENU_PADDING);

        ui.vertical_centered_justified(|ui| {
            if ui.button("View demo map").clicked() {
                params
                    .workspace_loader
                    .load_site(async move { LoadSite::from_data(&demo_office(), None) });
            }
            ui.add_space(MAIN_MENU_PADDING * 0.5);

            if ui.button("Open a file").clicked() {
                params.workspace_loader.load_from_dialog();
            }
            ui.add_space(MAIN_MENU_PADDING * 0.5);

            if ui.button("Create new file").clicked() {
                params.workspace_loader.create_empty_from_dialog();
            }
        });
    }
}

pub struct MainMenuPlugin;

impl Plugin for MainMenuPlugin {
    fn build(&self, app: &mut App) {
        let id = app.world_mut().spawn(Name::new("Main Menu")).id();
        app.world_mut().insert_resource(MainMenu { id });
        app.add_plugins(MainMenuSectionPlugin::<EditorMenuSection>::new())
            .add_systems(Update, egui_ui.run_if(in_state(AppState::MainMenu)));
    }
}
