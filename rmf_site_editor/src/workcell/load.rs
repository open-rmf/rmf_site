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

use std::path::PathBuf;
use std::collections::HashMap;

use bevy::prelude::*;
use std::collections::HashSet;
// TODO(luca) this shouldn't be site specific but shared
use crate::site::{AnchorBundle, ChangeCurrentWorkspace, DefaultFile, Dependents, NameInSite, SiteState};

use rmf_site_format::{Category, SiteID};

pub struct LoadWorkcell {
    /// The site data to load
    pub workcell: rmf_site_format::Workcell,
    /// Should the application switch focus to this new site
    pub focus: bool,
    /// Set if the workcell was loaded from a file
    pub default_file: Option<PathBuf>,
}

fn generate_workcell_entities(
    commands: &mut Commands,
    workcell: &rmf_site_format::Workcell,
) -> Entity {
    // Create hashmap of ids to entity to correctly generate hierarchy
    let mut id_to_entity = HashMap::new();
    // Hashmap of parent id to list of its children entities
    let mut parent_to_child_entities = HashMap::new();

    for (id, parented_anchor) in &workcell.anchors {
        let e = commands.spawn(AnchorBundle::new(parented_anchor.bundle.clone()).visible(true))
            .insert(SiteID(*id))
            .id();
        let mut child_entities: &mut Vec<Entity> = parent_to_child_entities.entry(parented_anchor.parent).or_default();
        child_entities.push(e);
        id_to_entity.insert(id, e);
    }

    for (id, parented_model) in &workcell.models {
        let e = commands.spawn(parented_model.bundle.clone())
            .insert(SiteID(*id))
            .id();
        // TODO(luca) this hashmap update is duplicated, refactor into function
        let mut child_entities: &mut Vec<Entity> = parent_to_child_entities.entry(parented_model.parent).or_default();
        child_entities.push(e);
        id_to_entity.insert(id, e);
    }

    // TODO(luca) assign SiteID to workcell root
    let mut root = commands.spawn(SpatialBundle::VISIBLE_IDENTITY)
        .insert(workcell.properties.clone())
        .insert(Category::Workcell)
        .id();

    for (parent, children) in parent_to_child_entities {
        let parent = match parent {
            Some(parent) => {
                // Child of an entity
                if let Some(parent) = id_to_entity.get(&parent) {
                    parent
                }
                else {
                    println!("DEV error, didn't find matching entity for id {}", parent);
                    continue;
                }
            },
            None => {
                // Child of root
                &root
            },
        };
        commands.entity(*parent)
            .insert(Dependents(HashSet::from_iter(children.clone())))
            .push_children(&children);
        // Update dependents as well
        // TODO(luca) A system to synchronize dependents and children?
    }
    root
}


pub fn load_workcell(
    mut commands: Commands,
    mut load_workcells: EventReader<LoadWorkcell>,
    mut change_current_workspace: EventWriter<ChangeCurrentWorkspace>,
    mut site_display_state: ResMut<State<SiteState>>,
) {
    for cmd in load_workcells.iter() {
        let root = generate_workcell_entities(&mut commands, &cmd.workcell);
        if let Some(path) = &cmd.default_file {
            commands.entity(root).insert(DefaultFile(path.clone()));
        }

        if cmd.focus {
            change_current_workspace.send(ChangeCurrentWorkspace { root });

            /*
            if *site_display_state.current() == SiteState::Off {
                site_display_state.set(SiteState::Display).ok();
            }
            */
        }
    }
}
