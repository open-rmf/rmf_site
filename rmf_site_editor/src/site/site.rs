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

/// Used as a resource that keeps track of the current site entity
#[derive(Clone, Copy, Debug, Default, Deref, DerefMut)]
pub struct CurrentSite(pub Option<Entity>);

/// Used as an event to command that a new site should be made the current one
#[derive(Clone, Copy, Debug)]
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
    mut change_current_site: EventReader<ChangeCurrentSite>,
    mut current_site: ResMut<CurrentSite>,
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

    if let Some(cmd) = change_current_site.iter().last() {
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

        if current_site.0 != Some(cmd.site) {
            if let Some(previous_site) = current_site.0 {
                set_visibility(previous_site, false);
            }
            set_visibility(cmd.site, true);
            current_site.0 = Some(cmd.site);
        }

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

pub fn site_display_on(
    current_site: Res<CurrentSite>,
    mut visibility: Query<&mut Visibility, With<SiteProperties>>,
) {
    if let Some(current_site) = current_site.0 {
        if let Ok(mut v) = visibility.get_mut(current_site) {
            v.is_visible = true;
        }
    }
}

pub fn site_display_off(
    current_site: Res<CurrentSite>,
    mut visibility: Query<&mut Visibility, With<SiteProperties>>,
) {
    if let Some(current_site) = current_site.0 {
        if let Ok(mut v) = visibility.get_mut(current_site) {
            v.is_visible = false;
        }
    }
}
