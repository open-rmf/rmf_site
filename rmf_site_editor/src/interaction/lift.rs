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
    site::{LiftDoorPlacemat, CurrentLevel, ToggleLiftDoorAvailability},
    interaction::*,
};
use bevy::prelude::*;

pub fn make_lift_placemat_gizmo(
    mut commands: Commands,
    mut new_placemats: Query<(Entity, &LiftDoorPlacemat, &mut Visibility, &mut Handle<StandardMaterial>), Added<LiftDoorPlacemat>>,
    current_level: Res<CurrentLevel>,
    assets: Res<InteractionAssets>,
) {
    for (e, placemat, mut visible, mut material) in &mut new_placemats {
        let materials = assets.lift_placemat_materials(placemat.door_available);
        *material = materials.passive.clone();
        commands.entity(e).insert(Gizmo::new().with_materials(materials));

        if Some(placemat.on_level) == current_level.0 {
            visible.is_visible = true;
        }
    }
}

pub fn handle_lift_placemat_clicks(
    placemats: Query<&LiftDoorPlacemat>,
    mut clicks: EventReader<GizmoClicked>,
    mut toggle: EventWriter<ToggleLiftDoorAvailability>,
) {
    for click in clicks.iter() {
        if let Ok(placemat) = placemats.get(click.0) {
            toggle.send(placemat.toggle_availability());
        }
    }
}

pub fn update_placemats_for_level_change(
    mut placemats: Query<(&LiftDoorPlacemat, &mut Visibility)>,
    current_level: Res<CurrentLevel>,
) {
    if current_level.is_changed() {
        for (placemat, mut visibility) in &mut placemats {
            visibility.is_visible = Some(placemat.on_level) == current_level.0;
        }
    }
}
