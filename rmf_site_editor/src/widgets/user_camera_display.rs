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
use bevy_egui::EguiContexts;
use crate::{AppState, widgets::RenderUiSet};

/// This resource keeps track of the region that the user camera display is
/// occupying in the window.
#[derive(Resource, Clone, Default)]
pub struct UserCameraDisplay {
    pub region: Rect,
}

/// Add the [`UserCameraDisplay`] resource to the application, along with the
/// system that updates it.
#[derive(Default)]
pub struct UserCameraDisplayPlugin {}

impl Plugin for UserCameraDisplayPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<UserCameraDisplay>()
            .add_systems(
                Update,
                update_user_camera_display
                .in_set(UserCameraDisplaySet)
                .after(RenderUiSet)
                .run_if(AppState::in_site_mode())
            );
    }
}

/// This set is for systems that update fields of the [`UserCameraDisplay`]
/// resource.
#[derive(SystemSet, Hash, PartialEq, Eq, Debug, Clone)]
pub struct UserCameraDisplaySet;

fn update_user_camera_display(
    mut user_camera_display: ResMut<UserCameraDisplay>,
    mut egui_contexts: EguiContexts,
) {
    let available_rect = egui_contexts.ctx_mut().available_rect();
    user_camera_display.region = Rect::new(
        available_rect.min.x,
        available_rect.min.y,
        available_rect.max.x,
        available_rect.max.y
    );
}
