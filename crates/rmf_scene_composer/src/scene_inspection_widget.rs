/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use crate::*;
use bevy::prelude::*;
use librmf_site_editor::widgets::prelude::*;
use rmf_site_egui::WidgetSystem;

#[derive(Default)]
pub struct SceneInspectionPlugin {}

impl Plugin for SceneInspectionPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins(InspectionPlugin::<InspectScene>::new());
    }
}

#[derive(SystemParam)]
pub struct InspectScene<'w, 's> {
    subscriber: SceneSubscriber<'w, 's>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectScene<'w, 's> {
    fn show(input: Inspect, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) -> () {
        let Inspect { selection, .. } = input;
        let mut params = state.get_mut(world);
        let Some(scene) = params.subscriber.get_subscription(selection) else {
            return;
        };

        let mut topic = scene.topic_name().to_owned();
        ui.horizontal(|ui| {
            ui.label("Subscription:");
            ui.text_edit_singleline(&mut topic);
        });

        if topic != scene.topic_name() {
            params.subscriber.change_subscription(selection, topic);
        }
    }
}
