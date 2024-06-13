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
use bevy_egui::egui::DragValue;
use crate::{
    site::*,
    widgets::{prelude::*, inspector::*},
    interaction::MoveTo,
    CurrentWorkspace,
};

#[derive(SystemParam)]
pub struct InspectGeography<'w, 's> {
    geographical: Query<'w, 's, &'static GeographicComponent>,
    tfs: Query<
        'w,
        's,
        &'static Transform,
        Or<(With<Anchor>, With<Pose>)>,
    >,
    move_to: EventWriter<'w, MoveTo>,
    current_workspace: Res<'w, CurrentWorkspace>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectGeography<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World
    ) {
        let mut param = state.get_mut(world);
        let Some(workspace) = param.current_workspace.root else {
            return;
        };
        let Ok(geo) = param.geographical.get(workspace) else {
            return;
        };
        let Ok(tf) = param.tfs.get(selection) else {
            return;
        };

        if let Some(offset) = geo.0 {
            let (mut lat, mut lon) = match world_to_latlon(
                tf.translation, offset.anchor,
            ) {
                Ok(values) => values,
                Err(err) => {
                    warn!("Unable to obtain latitude and longitude: {err:?}");
                    return;
                }
            };

            let old_lat = lat;
            let old_lon = lon;

            ui.label("Latitude");
            ui.add(DragValue::new(&mut lat).speed(0.0));
            ui.label("Longitude");
            ui.add(DragValue::new(&mut lon).speed(0.0));

            if old_lat != lat || old_lon != lon {
                param.move_to.send(MoveTo {
                    entity: selection,
                    transform: Transform::from_translation(
                        latlon_to_world(lat as f32, lon as f32, offset.anchor)
                    ),
                });
            }
        }
    }
}
