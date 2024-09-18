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
    interaction::*,
    site::{CurrentLevel, LiftCabin, LiftDoormat, ToggleLiftDoorAvailability},
};
use bevy::prelude::*;

pub fn make_lift_doormat_gizmo(
    mut commands: Commands,
    mut new_doormats: Query<
        (
            Entity,
            &LiftDoormat,
            &mut Visibility,
            &mut Handle<StandardMaterial>,
        ),
        Added<LiftDoormat>,
    >,
    current_level: Res<CurrentLevel>,
    assets: Res<InteractionAssets>,
) {
    for (e, doormat, mut visible, mut material) in &mut new_doormats {
        let materials = assets.lift_doormat_materials(doormat.door_available);
        *material = materials.passive.clone();
        commands
            .entity(e)
            .insert(Gizmo::new().with_materials(materials));

        if Some(doormat.on_level) == current_level.0 {
            *visible = Visibility::Inherited;
        }
    }
}

pub fn handle_lift_doormat_clicks(
    doormats: Query<&LiftDoormat>,
    mut clicks: EventReader<GizmoClicked>,
    mut toggle: EventWriter<ToggleLiftDoorAvailability>,
    mut select: EventWriter<Select>,
) {
    for click in clicks.read() {
        if let Ok(doormat) = doormats.get(click.0) {
            toggle.send(doormat.toggle_availability());
            select.send(Select::new(Some(doormat.for_lift)));
        }
    }
}

pub fn dirty_changed_lifts(mut lifts: Query<&mut Hovered, Changed<LiftCabin<Entity>>>) {
    for mut lift in &mut lifts {
        // This is a hack to force the outline to re-render after the lift cabin
        // has been reconstructed since any changes to the lift cabin will lazily
        // throw away all the entities that were previously constructed for it,
        // and spawn entirely new entities.
        lift.set_changed();
    }
}

pub fn update_doormats_for_level_change(
    mut doormats: Query<(&LiftDoormat, &mut Visibility)>,
    current_level: Res<CurrentLevel>,
) {
    if current_level.is_changed() {
        for (doormat, mut visibility) in &mut doormats {
            *visibility = if Some(doormat.on_level) == current_level.0 {
                Visibility::Inherited
            } else {
                Visibility::Hidden
            };
        }
    }
}
