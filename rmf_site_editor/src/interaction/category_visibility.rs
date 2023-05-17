/*
 * Copyright (C) 2023 Open Source Robotics Foundation
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

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;
use bevy::utils::HashSet;

use crate::{site::CurrentLevel, CurrentWorkspace};

use rmf_site_format::{
    DoorMarker, FloorMarker, LaneMarker, LiftCabin, LiftCabinDoorMarker, LocationTags,
    MeasurementMarker, ModelMarker, WallMarker,
};

#[derive(Clone, Debug, PartialEq)]
pub struct CategoryFlags {
    pub doors: bool,
    pub floors: bool,
    pub lanes: bool,
    pub lifts: bool,
    pub locations: bool,
    pub models: bool,
    pub measurements: bool,
    pub walls: bool,
}

// In normal site editor we want to display everything except measurements
impl Default for CategoryFlags {
    fn default() -> Self {
        Self {
            doors: true,
            floors: true,
            lanes: true,
            lifts: true,
            locations: true,
            models: true,
            measurements: true,
            walls: true,
        }
    }
}

/// Denote whether a certain category is visible or not
#[derive(Default, Debug, Deref, DerefMut, Resource)]
pub struct VisibilityCategoriesSettings(pub CategoryFlags);

/// Stored to verify which fields were changed between iterations
#[derive(Default, Debug, Deref, DerefMut, Resource)]
pub struct RecallVisibilityCategoriesSettings(pub CategoryFlags);

/// Keeps track of the entities that have been hidden by the user
#[derive(Default, Debug, Resource)]
pub struct VisibilitySettingsHiddenEntities {
    pub current_root: Option<Entity>,
    pub hidden: HashSet<Entity>,
}

#[derive(SystemParam)]
pub struct FilterParams<'w, 's> {
    doors: Query<'w, 's, Entity, With<DoorMarker>>,
    floors: Query<'w, 's, Entity, With<FloorMarker>>,
    lanes: Query<'w, 's, Entity, With<LaneMarker>>,
    lifts: Query<'w, 's, Entity, Or<(With<LiftCabin<Entity>>, With<LiftCabinDoorMarker>)>>,
    locations: Query<'w, 's, Entity, With<LocationTags>>,
    walls: Query<'w, 's, Entity, With<WallMarker>>,
    models: Query<'w, 's, Entity, With<ModelMarker>>,
    measurements: Query<'w, 's, Entity, With<MeasurementMarker>>,
    visibilities: Query<'w, 's, &'static mut Visibility>,
    categories_settings: Res<'w, VisibilityCategoriesSettings>,
    recall_categories_settings: ResMut<'w, RecallVisibilityCategoriesSettings>,
    hidden_res: ResMut<'w, VisibilitySettingsHiddenEntities>,
    current_level: Res<'w, CurrentLevel>,
    current_workspace: Res<'w, CurrentWorkspace>,
}

fn update_visibility(
    enabled: bool,
    mut visibilities: &mut Query<&mut Visibility>,
    entities: Vec<Entity>,
    mut hidden_set: &mut HashSet<Entity>,
) {
    for e in entities.iter() {
        if let Ok(mut vis) = visibilities.get_mut(*e) {
            if vis.is_visible && !enabled {
                vis.is_visible = false;
                hidden_set.insert(*e);
            } else if enabled && hidden_set.remove(e) {
                vis.is_visible = true;
            }
        }
    }
}

pub fn update_entity_category_visibilities(mut params: FilterParams) {
    // If the site or workspace was changed, reset the hidden cache
    let mut update = false;
    if **params.current_level != params.hidden_res.current_root
        && params.current_workspace.root != params.hidden_res.current_root
    {
        // TODO(luca) take workspace display into account
        params.hidden_res.hidden = HashSet::new();
        params.hidden_res.current_root = params.current_level.or(params.current_workspace.root);
        update = true;
    }
    if params.categories_settings.is_changed()
        && **params.categories_settings != **params.recall_categories_settings
    {
        update = true;
    }
    if update {
        if params.categories_settings.doors != params.recall_categories_settings.doors {
            update_visibility(
                params.categories_settings.doors,
                &mut params.visibilities,
                params.doors.iter().collect::<Vec<_>>(),
                &mut params.hidden_res.hidden,
            );
        }
        if params.categories_settings.floors != params.recall_categories_settings.floors {
            update_visibility(
                params.categories_settings.floors,
                &mut params.visibilities,
                params.floors.iter().collect::<Vec<_>>(),
                &mut params.hidden_res.hidden,
            );
        }
        if params.categories_settings.lanes != params.recall_categories_settings.lanes {
            update_visibility(
                params.categories_settings.lanes,
                &mut params.visibilities,
                params.lanes.iter().collect::<Vec<_>>(),
                &mut params.hidden_res.hidden,
            );
        }
        if params.categories_settings.lifts != params.recall_categories_settings.lifts {
            update_visibility(
                params.categories_settings.lifts,
                &mut params.visibilities,
                params.lifts.iter().collect::<Vec<_>>(),
                &mut params.hidden_res.hidden,
            );
        }
        if params.categories_settings.locations != params.recall_categories_settings.locations {
            update_visibility(
                params.categories_settings.locations,
                &mut params.visibilities,
                params.locations.iter().collect::<Vec<_>>(),
                &mut params.hidden_res.hidden,
            );
        }
        if params.categories_settings.measurements != params.recall_categories_settings.measurements
        {
            update_visibility(
                params.categories_settings.measurements,
                &mut params.visibilities,
                params.measurements.iter().collect::<Vec<_>>(),
                &mut params.hidden_res.hidden,
            );
        }
        if params.categories_settings.models != params.recall_categories_settings.models {
            update_visibility(
                params.categories_settings.models,
                &mut params.visibilities,
                params.models.iter().collect::<Vec<_>>(),
                &mut params.hidden_res.hidden,
            );
        }
        if params.categories_settings.walls != params.recall_categories_settings.walls {
            update_visibility(
                params.categories_settings.walls,
                &mut params.visibilities,
                params.walls.iter().collect::<Vec<_>>(),
                &mut params.hidden_res.hidden,
            );
        }
        **params.recall_categories_settings = params.categories_settings.0.clone();
    }
}
