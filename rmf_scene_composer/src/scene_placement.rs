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

use bevy::{
    prelude::*,
    ecs::system::SystemParam,
};
use bevy_impulse::prelude::*;

use librmf_site_editor::{
    interaction::*,
    keyboard::KeyboardServices,
    site::{Pose, CurrentLevel},
};

use anyhow::anyhow;

#[derive(Default)]
pub(crate) struct ScenePlacementPlugin {}

impl Plugin for ScenePlacementPlugin {
    fn build(&self, app: &mut App) {
        let place_scene_2d = spawn_place_scene_2d_workflow(app);
        app.insert_resource(ScenePlacementServices { place_scene_2d });
    }
}

#[derive(SystemParam)]
pub struct ScenePlacement<'w, 's> {
    services: Res<'w, ScenePlacementServices>,
    commands: Commands<'w, 's>,
}

impl<'w, 's> ScenePlacement<'w, 's> {
    pub fn place_scene(&mut self, scene_root: Entity) {
        let state = self
            .commands
            .spawn(SelectorInput(PlaceScene2d { scene_root }))
            .id();

        let run = RunSelector {
            selector: self.services.place_scene_2d,
            input: Some(state),
        };

        self.commands.add(move |world: &mut World| {
            world.send_event(run);
        })
    }
}

pub const PLACE_SCENE_2D_MODE_LABEL: &'static str = "place_scene_2d";

#[derive(Resource, Clone)]
pub(crate) struct ScenePlacementServices {
    pub place_scene_2d: Service<Option<Entity>, ()>,
}

pub struct PlaceScene2d {
    pub scene_root: Entity,
}

fn spawn_place_scene_2d_workflow(app: &mut App) -> Service<Option<Entity>, ()> {
    let setup = app.spawn_service(place_scene_2d_setup.into_blocking_service());
    let find_placement = app.world.resource::<ObjectPlacementServices>().find_placement_2d;
    let placement_chosen = app.spawn_service(on_scene_placement_chosen_2d.into_blocking_service());
    let handle_key_code = app.world.resource::<ObjectPlacementServices>().on_key_code_2d;
    let cleanup = app.spawn_service(place_scene_2d_cleanup.into_blocking_service());

    let keyboard_just_pressed = app
        .world
        .resource::<KeyboardServices>()
        .keyboard_just_pressed;

    app.world.spawn_io_workflow(build_2d_placement_workflow(
        setup,
        find_placement,
        placement_chosen,
        handle_key_code,
        cleanup,
        keyboard_just_pressed,
    ))
}

fn place_scene_2d_setup(
    In(key): In<BufferKey<PlaceScene2d>>,
    access: BufferAccess<PlaceScene2d>,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut commands: Commands,
    mut gizmo_blockers: ResMut<GizmoBlockers>,
    mut highlight: ResMut<HighlightAnchors>,
) -> SelectionNodeResult {
    let access = access.get(&key).or_broken_buffer()?;
    let state = access.newest().or_broken_buffer()?;

    set_visibility(cursor.dagger, &mut visibility, false);
    set_visibility(cursor.halo, &mut visibility, false);
    cursor.add_mode(PLACE_SCENE_2D_MODE_LABEL, &mut visibility);

    highlight.0 = false;
    gizmo_blockers.selecting = true;

    commands.get_entity(state.scene_root).or_broken_query()?.set_parent(cursor.frame);

    Ok(())
}

fn place_scene_2d_cleanup(
    In(key): In<BufferKey<PlaceScene2d>>,
    access: BufferAccess<PlaceScene2d>,
    mut commands: Commands,
    mut cursor: ResMut<Cursor>,
    mut visibility: Query<&mut Visibility>,
    mut gizmo_blockers: ResMut<GizmoBlockers>,
) {
    if let Ok(access) = access.get(&key) {
        if let Some(state) = access.newest() {
            // If this hasn't been removed from the buffer then it hasn't been
            // placed in the site and we should despawn it entirely.
            commands.entity(state.scene_root).despawn_recursive();
        }
    }

    cursor.remove_mode(PLACE_SCENE_2D_MODE_LABEL, &mut visibility);
    gizmo_blockers.selecting = false;
}

fn on_scene_placement_chosen_2d(
    In((placement, key)): In<(Transform, BufferKey<PlaceScene2d>)>,
    mut access: BufferAccessMut<PlaceScene2d>,
    mut commands: Commands,
    current_level: Res<CurrentLevel>,
) -> SelectionNodeResult {
    let Some(level) = current_level.0 else {
        return Err(Some(anyhow!("No current level is active, unable to place scene")));
    };

    let mut access = access.get_mut(&key).or_broken_buffer()?;
    let state = access.pull().or_broken_state()?;

    let mut scene = commands.get_entity(state.scene_root).or_broken_state()?;
    let pose: Pose = placement.into();
    scene.set_parent(level);
    scene.insert((pose, placement));

    Ok(())
}
