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
use super::{EditTask, TaskWidget};
use crate::{
    site::{update_task_kind_component, Task, TaskKind, TaskKinds},
    widgets::prelude::*,
};
use bevy::{
    ecs::{
        hierarchy::ChildOf,
        system::{SystemParam, SystemState},
    },
    prelude::*,
};
use bevy_egui::egui::DragValue;
use rmf_site_egui::*;
use rmf_site_format::WaitFor;

#[derive(Default)]
pub struct WaitForPlugin {}

impl Plugin for WaitForPlugin {
    fn build(&self, app: &mut App) {
        app.world_mut().resource_mut::<TaskKinds>().0.insert(
            WaitFor::label(),
            (
                |mut e_cmd| {
                    e_cmd.insert(WaitFor::default());
                },
                |mut e_cmd| {
                    e_cmd.remove::<WaitFor>();
                },
                |e, world| world.entity(e).get::<WaitFor>().is_some(),
            ),
        );
        let widget = Widget::<Tile>::new::<ViewWaitFor>(&mut app.world_mut());
        let task_widget = app.world().resource::<TaskWidget>().get();
        app.world_mut().spawn(widget).insert(ChildOf(task_widget));
        app.add_systems(PostUpdate, update_task_kind_component::<WaitFor>);
    }
}

#[derive(SystemParam)]
pub struct ViewWaitFor<'w, 's> {
    edit_task: Res<'w, EditTask>,
    tasks: Query<'w, 's, (&'static mut WaitFor, &'static mut Task)>,
}

impl<'w, 's> WidgetSystem<Tile> for ViewWaitFor<'w, 's> {
    fn show(_: Tile, ui: &mut Ui, state: &mut SystemState<Self>, world: &mut World) {
        let mut params = state.get_mut(world);

        let Some((mut wait_for, mut task)) =
            params.edit_task.and_then(|e| params.tasks.get_mut(e).ok())
        else {
            return;
        };

        let mut new_wait_for = wait_for.clone();
        ui.horizontal(|ui| {
            ui.label("Duration:");
            ui.add(
                DragValue::new(&mut new_wait_for.duration)
                    .range(0_f32..=std::f32::INFINITY)
                    .speed(1),
            );
            ui.label(" seconds");
        });

        if *wait_for != new_wait_for {
            *wait_for = new_wait_for.clone();

            if let Ok(description) = serde_json::to_value(new_wait_for.clone()) {
                *task.request_mut().description_mut() = description;
                *task.request_mut().description_display_mut() =
                    Some(format!("{}", new_wait_for.clone()));
            }
        }
    }
}
