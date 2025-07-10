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

use bevy_app::{App, Last, Plugin};
use bevy_ecs::prelude::*;
use bevy_impulse::*;
use bevy_input::prelude::*;

pub struct KeyboardServicePlugin;

impl Plugin for KeyboardServicePlugin {
    fn build(&self, app: &mut App) {
        let keyboard_just_pressed =
            app.spawn_continuous_service(Last, keyboard_just_pressed_stream);

        app.insert_resource(KeyboardServices {
            keyboard_just_pressed,
        });
    }
}

fn keyboard_just_pressed_stream(
    In(ContinuousService { key }): ContinuousServiceInput<(), (), StreamOf<KeyCode>>,
    mut orders: ContinuousQuery<(), (), StreamOf<KeyCode>>,
    keyboard_input: Res<ButtonInput<KeyCode>>,
) {
    let Some(mut orders) = orders.get_mut(&key) else {
        return;
    };

    if orders.is_empty() {
        return;
    }

    for key_code in keyboard_input.get_just_pressed() {
        orders.for_each(|order| order.streams().send(StreamOf(*key_code)));
    }
}

#[derive(Resource)]
pub struct KeyboardServices {
    pub keyboard_just_pressed: Service<(), (), StreamOf<KeyCode>>,
}
