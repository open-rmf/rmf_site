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
use crate::workcell::{ChangeCurrentWorkcell};
use crate::site::{AnchorBundle, DefaultFile, Dependents, PreventDeletion, SiteState};

use rmf_site_format::{Category, ConstraintDependents, MeshConstraint, NameInWorkcell, SiteID, WorkcellCollisionMarker, WorkcellVisualMarker};

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
    // Hashmap of parent model entity to constraint dependent entity
    let mut model_to_constraint_dependent_entities = HashMap::new();

    let root = commands.spawn(SpatialBundle::VISIBLE_IDENTITY)
        .insert(workcell.properties.clone())
        .insert(NameInWorkcell(workcell.properties.name.clone()))
        .insert(SiteID(workcell.id))
        .insert(Category::Workcell)
        .insert(PreventDeletion::because("Workcell root cannot be deleted".to_string()))
        .id();
    id_to_entity.insert(&workcell.id, root);

    for (id, parented_visual) in &workcell.visuals {
        let cmd = commands.spawn((SiteID(*id), WorkcellVisualMarker));
        let e = cmd.id();
        parented_visual.bundle.add_bevy_components(cmd);
        // TODO(luca) this hashmap update is duplicated, refactor into function
        let child_entities: &mut Vec<Entity> = parent_to_child_entities.entry(parented_visual.parent).or_default();
        child_entities.push(e);
        id_to_entity.insert(id, e);
    }

    for (id, parented_collision) in &workcell.collisions {
        let cmd = commands.spawn((SiteID(*id), WorkcellCollisionMarker));
        let e = cmd.id();
        parented_collision.bundle.add_bevy_components(cmd);
        // TODO(luca) this hashmap update is duplicated, refactor into function
        let child_entities: &mut Vec<Entity> = parent_to_child_entities.entry(parented_collision.parent).or_default();
        child_entities.push(e);
        id_to_entity.insert(id, e);
    }

    for (id, parented_anchor) in &workcell.frames {
        let e = commands.spawn(AnchorBundle::new(parented_anchor.bundle.anchor.clone()).visible(true))
            .insert(SiteID(*id))
            .id();
        if let Some(c) = &parented_anchor.bundle.mesh_constraint {
            let model_entity = *id_to_entity.get(&c.entity).expect("Mesh constraint refers to non existing model");
            commands.entity(e).insert(MeshConstraint {
                entity: model_entity,
                element: c.element.clone(),
                relative_pose: c.relative_pose,
            });
            let constraint_dependents: &mut HashSet<Entity> = model_to_constraint_dependent_entities.entry(model_entity).or_default();
            constraint_dependents.insert(e);
        }
        if let Some(name) = &parented_anchor.bundle.name {
            commands.entity(e).insert(name.clone());
        }
        let child_entities: &mut Vec<Entity> = parent_to_child_entities.entry(parented_anchor.parent).or_default();
        child_entities.push(e);
        id_to_entity.insert(id, e);
    }

    // Add constraint dependents to models
    for (model, dependents) in model_to_constraint_dependent_entities {
        commands.entity(model).insert(ConstraintDependents(dependents));
    }

    for (parent, children) in parent_to_child_entities {
        if let Some(parent) = id_to_entity.get(&parent) {
            commands.entity(*parent)
                .insert(Dependents(HashSet::from_iter(children.clone())))
                .push_children(&children);
        }
        else {
            println!("DEV error, didn't find matching entity for id {}", parent);
            continue;
        }
    }
    root
}

pub fn load_workcell(
    mut commands: Commands,
    mut load_workcells: EventReader<LoadWorkcell>,
    mut change_current_workcell: EventWriter<ChangeCurrentWorkcell>,
    mut site_display_state: ResMut<State<SiteState>>,
) {
    for cmd in load_workcells.iter() {
        println!("Loading workcell");
        let root = generate_workcell_entities(&mut commands, &cmd.workcell);
        if let Some(path) = &cmd.default_file {
            commands.entity(root).insert(DefaultFile(path.clone()));
        }

        if cmd.focus {
            change_current_workcell.send(ChangeCurrentWorkcell { root });

            // TODO(luca) get rid of SiteState
            if *site_display_state.current() == SiteState::Display {
                site_display_state.set(SiteState::Off).ok();
            }
        }
    }
}
