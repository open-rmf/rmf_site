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
    site::{Change, Distance},
    widgets::{prelude::*, Inspect, InspectOptionF32},
};
use bevy::prelude::*;

#[derive(SystemParam)]
pub struct InspectMeasurement<'w, 's> {
    distances: Query<'w, 's, &'static Distance>,
    change_distance: EventWriter<'w, Change<Distance>>,
}

impl<'w, 's> WidgetSystem<Inspect> for InspectMeasurement<'w, 's> {
    fn show(
        Inspect { selection, .. }: Inspect,
        ui: &mut Ui,
        state: &mut SystemState<Self>,
        world: &mut World,
    ) {
        let mut params = state.get_mut(world);
        let Ok(distance) = params.distances.get(selection) else {
            return;
        };

        if let Some(new_distance) = InspectOptionF32::new("Distance", distance.0, 10.0)
            .clamp_range(0.0..=10000.0)
            .min_decimals(2)
            .max_decimals(2)
            .speed(0.01)
            .suffix(" m")
            .show(ui)
        {
            params
                .change_distance
                .write(Change::new(Distance(new_distance), selection));
        }
        ui.add_space(10.0);
    }
}
