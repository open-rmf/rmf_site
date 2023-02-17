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
use rmf_site_format::{LevelProperties, SiteProperties, WorkcellProperties};
use std::collections::HashMap;

/// Used as a resource that keeps track of the current site entity
#[derive(Clone, Copy, Debug, Default, Resource)]
pub struct CurrentWorkspace {
    pub root: Option<Entity>,
    pub display: bool,
}

/// Used to keep track of visibility when switching workspace
#[derive(Debug, Default, Resource)]
pub struct RecallWorkspace(Option<Entity>);

impl CurrentWorkspace {
    pub fn to_site(self, open_sites: &Query<Entity, With<SiteProperties>>) -> Option<Entity> {
        let site_entity = self.root?;
        open_sites.get(site_entity).ok()
    }
}

/// Used as an event to command that a new site should be made the current one
#[derive(Clone, Copy, Debug)]
pub struct ChangeCurrentWorkspace {
    /// What should the current site be
    pub root: Entity,
}

/// Used as a resource that keeps track of the current level entity
#[derive(Clone, Copy, Debug, Default, Deref, DerefMut, Resource)]
pub struct CurrentLevel(pub Option<Entity>);

/// Used as a resource that maps from the site entity to the level entity which
/// was most recently selected for it.
#[derive(Clone, Debug, Default, Resource)]
pub struct CachedLevels(pub HashMap<Entity, Entity>);

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
    open_workspaces: Query<Entity, Or<(With<SiteProperties>, With<WorkcellProperties>)>>,
    children: Query<&Children>,
    parents: Query<&Parent>,
    levels: Query<Entity, With<LevelProperties>>,
) {
    let mut set_visibility = |entity, value| {
        if let Ok(mut v) = visibility.get_mut(entity) {
            v.is_visible = value;
        }
    };

    if let Some(cmd) = change_current_workspace.iter().last() {
        if open_workspaces.get(cmd.root).is_err() {
            println!(
                "Requested workspace change to an entity that is not an open site or workcell: {:?}",
                cmd.root
            );
            return;
        }

        if current_workspace.root != Some(cmd.root) {
            current_workspace.root = Some(cmd.root);
            current_workspace.display = true;
        }

        // TODO(luca) Early return for workcell editor, probably splitting the open_workspaces
        // Query

        if let Some(cached_level) = cached_levels.0.get(&cmd.root) {
            set_visibility(*cached_level, true);
            current_level.0 = Some(*cached_level);
        } else {
            if let Ok(children) = children.get(cmd.root) {
                let mut found_level = false;
                for child in children {
                    if let Ok(level) = levels.get(*child) {
                        cached_levels.0.insert(cmd.root, level);
                        current_level.0 = Some(level);
                        found_level = true;
                        set_visibility(level, true);
                    }
                }

                if !found_level {
                    // Create a new blank level for the user
                    let new_level = commands.entity(cmd.root).add_children(|site| {
                        site.spawn(SpatialBundle::default())
                            .insert(LevelProperties {
                                name: "<unnamed level>".to_string(),
                                elevation: 0.,
                            })
                            .id()
                    });

                    cached_levels.0.insert(cmd.root, new_level);
                    current_level.0 = Some(new_level);
                }
            }
        }
    }
}

pub fn sync_workspace_visibility(
    current_workspace: Res<CurrentWorkspace>,
    mut recall: ResMut<RecallWorkspace>,
    mut visibility: Query<&mut Visibility>,
) {
    if !current_workspace.is_changed() {
        return;
    }

    if recall.0 != current_workspace.root {
        // Set visibility of current to target
        if let Some(current_workspace_entity) = current_workspace.root {
            if let Ok(mut v) = visibility.get_mut(current_workspace_entity) {
                v.is_visible = current_workspace.display;
            }
        }
        // Disable visibility in recall
        if let Some(recall) = recall.0 {
            if let Ok(mut v) = visibility.get_mut(recall) {
                v.is_visible = false;
            }
        }
        recall.0 = current_workspace.root;
    }
}
