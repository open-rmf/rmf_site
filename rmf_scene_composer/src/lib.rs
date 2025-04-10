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

mod scene_creation_widget;
use scene_creation_widget::*;

mod generate_scene;
use generate_scene::*;

mod protos;
pub use protos::*;

mod scene_subscription;
pub use scene_subscription::*;

mod scene_placement;
pub use scene_placement::*;

mod scene_site_extension;
pub use scene_site_extension::*;

mod scene_inspection_widget;
pub use scene_inspection_widget::*;

use bevy::prelude::*;

#[derive(Default)]
pub struct SceneComposerPlugin {}

impl Plugin for SceneComposerPlugin {
    fn build(&self, app: &mut App) {
        app.add_plugins((
            SceneSubscribingPlugin::default(),
            ScenePlacementPlugin::default(),
            SceneCreationPlugin::default(),
            SceneInspectionPlugin::default(),
            SceneSiteExtensionPlugin::default(),
        ));
    }
}

pub fn run(command_line_args: Vec<String>) {
    let mut app = App::new();
    app.add_plugins((
        librmf_site_editor::SiteEditor::from_cli_args(command_line_args),
        SceneComposerPlugin::default(),
    ));

    app.run();
}
