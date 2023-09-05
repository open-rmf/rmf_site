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

use crate::CurrentWorkspace;
use bevy::prelude::*;
use rmf_site_format::{LevelElevation, LevelProperties, NameInSite, NameOfSite};

/// Used as an event to command that a new site should be made the current one
#[derive(Clone, Copy, Debug, Event)]
pub struct ChangeCurrentSite {
    /// What should the current site be
    pub site: Entity,
    /// What should its current level be
    pub level: Option<Entity>,
}

/// Used as a resource that keeps track of the current level entity
#[derive(Clone, Copy, Debug, Default, Deref, DerefMut, Resource)]
pub struct CurrentLevel(pub Option<Entity>);

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
    mut current_workspace: ResMut<CurrentWorkspace>,
    mut current_level: ResMut<CurrentLevel>,
    cached_levels: Query<&CachedLevel>,
    mut visibility: Query<&mut Visibility>,
    open_sites: Query<Entity, With<NameOfSite>>,
    children: Query<&Children>,
    parents: Query<&Parent>,
    levels: Query<Entity, With<LevelElevation>>,
) {
    let mut set_visibility = |entity, value| {
        if let Ok(mut v) = visibility.get_mut(entity) {
            v = value;
        }
    };

    if let Some(cmd) = change_current_site.iter().last() {
        if open_sites.get(cmd.site).is_err() {
            warn!(
                "Requested workspace change to an entity that is not an open site: {:?}",
                cmd.site
            );
            return;
        }

        if let Some(chosen_level) = cmd.level {
            if parents
                .get(chosen_level)
                .ok()
                .filter(|parent| parent.get() == cmd.site)
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
                            .spawn(SpatialBundle::default())
                            .insert(LevelProperties {
                                name: NameInSite("<unnamed level>".to_owned()),
                                elevation: LevelElevation(0.),
                                global_floor_visibility: default(),
                                global_drawing_visibility: default(),
                            })
                            .set_parent(cmd.site)
                            .id();

                        commands.entity(cmd.site).insert(CachedLevel(new_level));
                        current_level.0 = Some(new_level);
                    }
                }
            }
        }
    }
}
