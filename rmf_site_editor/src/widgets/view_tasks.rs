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

use crate::{
    site::{
        Change, ChangeCurrentScenario, CurrentScenario, ModelMarker, NameInSite, Scenario,
        ScenarioMarker,
    },
    widgets::prelude::*,
    Icons,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{Button, CollapsingHeader, Color32, Ui};
use rmf_site_format::{Angle, ScenarioBundle, SiteID};

#[derive(Default)]
pub struct ViewTasksPlugin {}

impl Plugin for ViewTasksPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(PropertiesTilePlugin::<ViewTasks>::new());
    }
}

#[derive(SystemParam)]
pub struct ViewTasks<'w, 's> {
    commands: Commands<'w, 's>,
    children: Query<'w, 's, &'static Children>,
    scenarios: Query<
        'w,
        's,
        (Entity, &'static NameInSite, &'static Scenario<Entity>),
        With<ScenarioMarker>,
    >,
    change_name: EventWriter<'w, Change<NameInSite>>,
    change_current_scenario: EventWriter<'w, ChangeCurrentScenario>,
    current_scenario: ResMut<'w, CurrentScenario>,
    model_instances:
        Query<'w, 's, (Entity, &'static NameInSite, &'static SiteID), With<ModelMarker>>,
    icons: Res<'w, Icons>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewTasks<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        CollapsingHeader::new("Tasks")
            .default_open(true)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewTasks<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {}
}
