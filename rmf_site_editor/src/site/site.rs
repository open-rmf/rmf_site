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

use bevy::prelude::*;
use rmf_site_format::{LevelProperties, SiteProperties};
use std::collections::HashMap;

// TODO(luca) move to a workspace.rs file
/// Used as a resource that keeps track of the current workspace type and entity
#[derive(Clone, Copy, Debug, Default)]
pub struct CurrentWorkspace {
    pub root: Option<Entity>,
    pub display: bool,
}

impl CurrentWorkspace {
    // TODO(luca) there is probably a cleaner way to write this?
    pub fn top_entity(&self) -> Option<Entity> {
        match self {
            CurrentWorkspace::None => None,
            CurrentWorkspace::Site(e) => Some(*e),
            CurrentWorkspace::Workcell(e) => Some(*e),
        }
    }
}

/// Used as an event to command what workspace should be changed to
#[derive(Clone, Copy, Debug)]
pub enum ChangeCurrentWorkspace {
    None,
    Site(ChangeCurrentSite),
    // TODO(luca) make a ChangeCurrentWorkcell struct
    //Workcell(ChangeCurrentSitePrivate),
}

/// Used as an event to command that a new site should be made the current one
#[derive(Clone, Copy, Debug)]
// TODO(luca) back to previous name once refactoring is done
pub struct ChangeCurrentSite {
    /// What should the current site be
    pub site: Entity,
    /// What should its current level be
    pub level: Option<Entity>,
}

/// Used as a resource that keeps track of the current level entity
#[derive(Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct CurrentLevel(pub Option<Entity>);

/// Used as a resource that maps from the site entity to the level entity which
/// was most recently selected for it.
#[derive(Clone, Debug, Default)]
pub struct CachedLevels(pub HashMap<Entity, Entity>);

/// Used as a resource to keep track of all currently opened sites
#[derive(Clone, Debug, Default)]
pub struct OpenSites(pub Vec<Entity>);

/// This component is placed on the Site entity to keep track of what the next
/// SiteID should be when saving.
#[derive(Component, Clone, Copy, Debug)]
pub struct NextSiteID(pub u32);

pub fn change_site(
    mut commands: Commands,
    mut change_current_workspace: EventReader<ChangeCurrentWorkspace>,
    mut current_workspace: ResMut<CurrentWorkspace>,
    mut current_level: ResMut<CurrentLevel>,
    mut cached_levels: ResMut<CachedLevels>,
    mut visibility: Query<&mut Visibility>,
    open_sites: Res<OpenSites>,
    children: Query<&Children>,
    parents: Query<&Parent>,
    levels: Query<Entity, With<LevelProperties>>,
) {
    let mut set_visibility = |entity, value| {
        if let Ok(mut v) = visibility.get_mut(entity) {
            v.is_visible = value;
        }
    };

    let last_evt = change_current_workspace.iter().last();
    if let Some(evt @ ChangeCurrentWorkspace::Site(cmd)) = last_evt {
        if open_sites.0.iter().find(|s| **s == cmd.site).is_none() {
            println!(
                "Requested site change to an entity that is not an open site: {:?}",
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
                println!(
                    "Requested level change to an entity {:?} that is not a level of the requested site {:?}",
                    chosen_level,
                    cmd.site,
                );
                return;
            }
        }

        // If we are in a site
        if let CurrentWorkspace::Site(current_site) = *current_workspace {
            if current_site != cmd.site {
                set_visibility(current_site, false);
            }
            set_visibility(cmd.site, true);
        }
        *current_workspace = CurrentWorkspace::Site(cmd.site);

        if let Some(new_level) = cmd.level {
            if let Some(previous_level) = current_level.0 {
                if previous_level != new_level {
                    set_visibility(previous_level, false);
                }
            }

            set_visibility(new_level, true);
            cached_levels.0.insert(cmd.site, new_level);
            current_level.0 = Some(new_level);
        } else {
            if let Some(cached_level) = cached_levels.0.get(&cmd.site) {
                set_visibility(*cached_level, true);
                current_level.0 = Some(*cached_level);
            } else {
                if let Ok(children) = children.get(cmd.site) {
                    let mut found_level = false;
                    for child in children {
                        if let Ok(level) = levels.get(*child) {
                            cached_levels.0.insert(cmd.site, level);
                            current_level.0 = Some(level);
                            found_level = true;
                            set_visibility(level, true);
                        }
                    }

                    if !found_level {
                        // Create a new blank level for the user
                        let new_level = commands.entity(cmd.site).add_children(|site| {
                            site.spawn_bundle(SpatialBundle::default())
                                .insert(LevelProperties {
                                    name: "<unnamed level>".to_string(),
                                    elevation: 0.,
                                })
                                .id()
                        });

                        cached_levels.0.insert(cmd.site, new_level);
                        current_level.0 = Some(new_level);
                    }
                }
            }
        }
    }
}

// TODO(luca) add a boolean argument and merge the two functions below?
pub fn site_display_on(
    current_workspace: Res<CurrentWorkspace>,
    mut visibility: Query<&mut Visibility, With<SiteProperties>>,
) {
    if let CurrentWorkspace::Site(current_site) = *current_workspace {
        if let Ok(mut v) = visibility.get_mut(current_site) {
            v.is_visible = true;
        }
    }
}

pub fn site_display_off(
    current_workspace: Res<CurrentWorkspace>,
    mut visibility: Query<&mut Visibility, With<SiteProperties>>,
) {
    if let CurrentWorkspace::Site(current_site) = *current_workspace {
        if let Ok(mut v) = visibility.get_mut(current_site) {
            v.is_visible = false;
        }
    }
}
