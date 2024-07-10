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
    site::{
        Category, Change, CurrentScenario, Delete, DrawingMarker, FloorMarker, LevelElevation,
        LevelProperties, NameInSite, Scenario, ScenarioMarker,
    },
    widgets::{prelude::*, Icons},
    AppState, CurrentWorkspace, RecencyRanking,
};
use bevy::{ecs::system::SystemParam, prelude::*};
use bevy_egui::egui::{CollapsingHeader, DragValue, ImageButton, Ui};
use std::cmp::{Ordering, Reverse};

/// Add a plugin for viewing and editing a list of all levels
#[derive(Default)]
pub struct ViewScenariosPlugin {}

impl Plugin for ViewScenariosPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<ScenarioDisplay>()
            .add_plugins(PropertiesTilePlugin::<ViewScenarios>::new());
    }
}

#[derive(SystemParam)]
pub struct ViewScenarios<'w, 's> {
    scenarios: Query<
        'w,
        's,
        (Entity, &'static NameInSite, &'static Scenario<Entity>),
        With<ScenarioMarker>,
    >,
    icons: Res<'w, Icons>,
    display_scenarios: ResMut<'w, ScenarioDisplay>,
    current_scenario: ResMut<'w, CurrentScenario>,
    commands: Commands<'w, 's>,
    app_state: Res<'w, State<AppState>>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewScenarios<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let mut params = state.get_mut(world);
        CollapsingHeader::new("Scenarios")
            .default_open(true)
            .show(ui, |ui| {
                params.show_widget(ui);
            });
    }
}

impl<'w, 's> ViewScenarios<'w, 's> {
    pub fn show_widget(&mut self, ui: &mut Ui) {
        return;
    }
}

#[derive(Resource)]
pub struct ScenarioDisplay {
    pub new_name: String,
    pub order: Vec<Vec<Entity>>,
}

impl Default for ScenarioDisplay {
    fn default() -> Self {
        Self {
            new_name: "New Scenario".to_string(),
            order: Vec::new(),
        }
    }
}
