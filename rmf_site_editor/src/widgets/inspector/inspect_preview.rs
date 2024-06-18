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
    interaction::SpawnPreview,
    site::PreviewableMarker,
    widgets::{Inspect, prelude::*},
};
use bevy::prelude::*;

#[derive(SystemParam)]
pub struct InspectPreview<'w, 's> {
    previewable: Query<'w, 's, &'static PreviewableMarker>,
    spawn_preview: EventWriter<'w, SpawnPreview>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectPreview<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        if params.previewable.contains(selection) {
            if ui.button("Preview").clicked() {
                params.spawn_preview.send(SpawnPreview::new(Some(selection)));
            }
            ui.add_space(10.0);
        }
    }
}
