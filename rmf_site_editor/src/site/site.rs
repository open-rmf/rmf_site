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

use crate::{interaction::CameraControls, CurrentWorkspace};
use bevy::ecs::hierarchy::ChildOf;
use bevy::prelude::*;
use rmf_site_format::{
    LevelElevation, LevelProperties, NameInSite, NameOfSite, Pose, ScenarioMarker,
    UserCameraPoseMarker,
};

use super::{ChangeCurrentScenario, CreateScenario};

/// Used as an event to command that a new site should be made the current one
#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentSite {
    /// What should the current site be
    pub site: Entity,
    /// What should its current level be
    pub level: Option<Entity>,
    /// What should its current scenario be
    pub scenario: Option<Entity>,
}

/// Used as a resource that keeps track of the current level entity
#[derive(Clone, Copy, Debug, Default, Deref, DerefMut, Resource)]
pub struct CurrentLevel(pub Option<Entity>);

/// Used as a resource that keeps track of the current scenario entity
#[derive(Clone, Copy, Debug, Default, Deref, DerefMut, Resource)]
pub struct CurrentScenario(pub Option<Entity>);

/// Used as a component that maps from the site entity to the level entity which
/// was most recently selected for it.
#[derive(Component, Clone, Deref, DerefMut, Debug)]
pub struct CachedLevel(Entity);

/// This component is placed on the Site entity to keep track of what the next
/// SiteID should be when saving.
#[derive(Component, Clone, Copy, Debug)]
pub struct NextSiteID(pub u32);

pub fn change_site(
    mut commands: Commands,
    mut change_current_site: EventReader<ChangeCurrentSite>,
    mut change_current_scenario: EventWriter<ChangeCurrentScenario>,
    mut create_new_scenario: EventWriter<CreateScenario>,
    mut current_workspace: ResMut<CurrentWorkspace>,
    mut current_level: ResMut<CurrentLevel>,
    current_scenario: ResMut<CurrentScenario>,
    cached_levels: Query<&CachedLevel>,
    mut visibility: Query<&mut Visibility>,
    open_sites: Query<Entity, With<NameOfSite>>,
    children: Query<&Children>,
    child_of: Query<&ChildOf>,
    levels: Query<Entity, With<LevelElevation>>,
    scenarios: Query<Entity, With<ScenarioMarker>>,
) {
    let mut set_visibility = |entity, value| {
        if let Ok(mut v) = visibility.get_mut(entity) {
            *v = value;
        }
    };

    if let Some(cmd) = change_current_site.read().last() {
        if open_sites.get(cmd.site).is_err() {
            warn!(
                "Requested workspace change to an entity that is not an open site: {:?}",
                cmd.site
            );
            return;
        }

        if let Some(chosen_level) = cmd.level {
            if child_of
                .get(chosen_level)
                .ok()
                .filter(|child_of| child_of.parent() == cmd.site)
                .is_none()
            {
                warn!(
                    "Requested level change to an entity {:?} that is not a level of the requested site {:?}",
                    chosen_level,
                    cmd.site,
                );
                return;
            }
        }

        current_workspace.root = Some(cmd.site);
        current_workspace.display = true;

        if let Some(new_level) = cmd.level {
            if let Some(previous_level) = current_level.0 {
                if previous_level != new_level {
                    set_visibility(previous_level, Visibility::Hidden);
                }
            }

            set_visibility(new_level, Visibility::Inherited);
            commands.entity(cmd.site).insert(CachedLevel(new_level));
            current_level.0 = Some(new_level);
        } else {
            if let Ok(cached_level) = cached_levels.get(cmd.site) {
                set_visibility(**cached_level, Visibility::Inherited);
                current_level.0 = Some(**cached_level);
            } else {
                if let Ok(children) = children.get(cmd.site) {
                    let mut found_level = false;
                    for child in children {
                        if let Ok(level) = levels.get(*child) {
                            commands.entity(cmd.site).insert(CachedLevel(level));
                            current_level.0 = Some(level);
                            found_level = true;
                            set_visibility(level, Visibility::Inherited);
                        }
                    }

                    if !found_level {
                        // Create a new blank level for the user
                        let new_level = commands
                            .spawn((Transform::default(), Visibility::default()))
                            .insert(LevelProperties {
                                name: NameInSite("<unnamed level>".to_owned()),
                                elevation: LevelElevation(0.),
                                global_floor_visibility: default(),
                                global_drawing_visibility: default(),
                            })
                            .insert(ChildOf(cmd.site))
                            .id();

                        commands.entity(cmd.site).insert(CachedLevel(new_level));
                        current_level.0 = Some(new_level);
                    }
                }
            }
        }

        if let Some(new_scenario) = cmd.scenario {
            if let Some(previous_scenario) = current_scenario.0 {
                if previous_scenario != new_scenario {
                    change_current_scenario.write(ChangeCurrentScenario(new_scenario));
                }
            }
        } else {
            if let Ok(children) = children.get(cmd.site) {
                let any_scenario = children
                    .iter()
                    .filter(|child| scenarios.get(*child).is_ok())
                    .next();
                if let Some(new_scenario) = any_scenario {
                    change_current_scenario.write(ChangeCurrentScenario(new_scenario));
                } else {
                    create_new_scenario.write(CreateScenario {
                        name: None,
                        parent: None,
                    });
                }
            }
        }
    }
}

pub fn set_camera_transform_for_changed_site(
    current_workspace: Res<CurrentWorkspace>,
    current_level: Res<CurrentLevel>,
    mut camera_controls: ResMut<CameraControls>,
    children: Query<&Children>,
    user_camera_poses: Query<&Pose, With<UserCameraPoseMarker>>,
    mut transforms: Query<&mut Transform>,
) {
    if current_workspace.is_changed() {
        let Some(level) = current_level.0 else {
            return;
        };

        // TODO(luca) Add an actual default pose rather than first in query
        if let Some(pose) = children
            .get(level)
            .ok()
            .and_then(|children| children.iter().find_map(|c| user_camera_poses.get(c).ok()))
        {
            if let Ok(mut tf) = transforms.get_mut(camera_controls.perspective_camera_entities[0]) {
                *tf = pose.transform();
            }
            let mut translation = pose.transform().translation;
            // TODO(luca) these are the same value that are in rmf_site_format, should we change
            // them?
            translation.x = translation.x + 10.0;
            translation.y = translation.y + 10.0;
            translation.z = 0.0;
            camera_controls.orbit_center = Some(translation);
        }
    }
}
