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

use std::collections::HashMap;
use std::path::PathBuf;

use crate::{
    site::{
        AnchorBundle, CollisionMeshMarker, DefaultFile, Dependents, ModelLoader, PreventDeletion,
        VisualMeshMarker,
    },
    workcell::ChangeCurrentWorkcell,
    WorkspaceMarker,
};
use bevy::prelude::*;
use std::collections::HashSet;

use rmf_site_format::{
    Category, FrameMarker, Geometry, ModelMarker, NameInWorkcell, Parented, Scale, SiteID,
    Workcell, WorkcellModel,
};

#[derive(Event)]
pub struct LoadWorkcell {
    /// The site data to load
    pub workcell: Workcell,
    /// Should the application switch focus to this new site
    pub focus: bool,
    /// Set if the workcell was loaded from a file
    pub default_file: Option<PathBuf>,
}

fn generate_workcell_entities(
    commands: &mut Commands,
    workcell: &Workcell,
    model_loader: &mut ModelLoader,
) -> Entity {
    // Create hashmap of ids to entity to correctly generate hierarchy
    let mut id_to_entity = HashMap::new();
    // Hashmap of parent id to list of its children entities
    let mut parent_to_child_entities = HashMap::new();

    let root = commands
        .spawn(SpatialBundle::INHERITED_IDENTITY)
        .insert(workcell.properties.clone())
        .insert(SiteID(workcell.id))
        .insert(Category::Workcell)
        .insert(WorkspaceMarker)
        .insert(PreventDeletion::because(
            "Workcell root cannot be deleted".to_string(),
        ))
        .id();
    id_to_entity.insert(workcell.id, root);

    let mut add_model =
        |parented: &Parented<u32, WorkcellModel>, id: u32, e: Entity, commands: &mut Commands| {
            match &parented.bundle.geometry {
                Geometry::Primitive(primitive) => {
                    commands.entity(e).insert((
                        primitive.clone(),
                        parented.bundle.pose.clone(),
                        NameInWorkcell(parented.bundle.name.clone()),
                    ));
                }
                Geometry::Mesh { source, scale } => {
                    commands.entity(e).insert((
                        NameInWorkcell(parented.bundle.name.clone()),
                        parented.bundle.pose.clone(),
                        Scale(scale.unwrap_or(Vec3::ONE)),
                        ModelMarker,
                    ));
                    model_loader.update_asset_source(e, source.clone());
                }
            };
            commands.entity(e).insert(SiteID(id));
            let child_entities: &mut Vec<Entity> =
                parent_to_child_entities.entry(parented.parent).or_default();
            child_entities.push(e);
            id_to_entity.insert(id, e);
        };

    for (id, visual) in &workcell.visuals {
        let e = commands.spawn((VisualMeshMarker, Category::Visual)).id();
        add_model(visual, *id, e, commands);
    }

    for (id, collision) in &workcell.collisions {
        let e = commands
            .spawn((CollisionMeshMarker, Category::Collision))
            .id();
        add_model(collision, *id, e, commands);
    }

    for (id, parented_anchor) in &workcell.frames {
        let e = commands
            .spawn(AnchorBundle::new(parented_anchor.bundle.anchor.clone()).visible(true))
            .insert(SiteID(*id))
            .insert(FrameMarker)
            .id();
        if let Some(name) = &parented_anchor.bundle.name {
            commands.entity(e).insert(name.clone());
        }
        let child_entities: &mut Vec<Entity> = parent_to_child_entities
            .entry(parented_anchor.parent)
            .or_default();
        child_entities.push(e);
        id_to_entity.insert(*id, e);
    }

    for (id, parented_inertia) in &workcell.inertias {
        let e = commands
            .spawn(SpatialBundle::INHERITED_IDENTITY)
            .insert(parented_inertia.bundle.clone())
            .insert(Category::Inertia)
            .insert(SiteID(*id))
            .id();
        let child_entities: &mut Vec<Entity> = parent_to_child_entities
            .entry(parented_inertia.parent)
            .or_default();
        child_entities.push(e);
        id_to_entity.insert(*id, e);
    }

    for (id, parented_joint) in &workcell.joints {
        let joint = &parented_joint.bundle;
        let mut cmd = commands.spawn(SiteID(*id));
        let e = cmd.id();
        joint.add_bevy_components(&mut cmd);
        let child_entities: &mut Vec<Entity> = parent_to_child_entities
            .entry(parented_joint.parent)
            .or_default();
        child_entities.push(e);
        id_to_entity.insert(*id, e);
    }

    for (parent, children) in parent_to_child_entities {
        if let Some(parent) = id_to_entity.get(&parent) {
            commands
                .entity(*parent)
                .insert(Dependents(HashSet::from_iter(children.clone())))
                .push_children(&children);
        } else {
            error!("DEV error, didn't find matching entity for id {}", parent);
            continue;
        }
    }
    root
}

pub fn load_workcell(
    mut commands: Commands,
    mut load_workcells: EventReader<LoadWorkcell>,
    mut change_current_workcell: EventWriter<ChangeCurrentWorkcell>,
    mut model_loader: ModelLoader,
) {
    for cmd in load_workcells.read() {
        info!("Loading workcell");
        let root = generate_workcell_entities(&mut commands, &cmd.workcell, &mut model_loader);
        if let Some(path) = &cmd.default_file {
            commands.entity(root).insert(DefaultFile(path.clone()));
        }

        if cmd.focus {
            change_current_workcell.send(ChangeCurrentWorkcell { root });
        }
    }
}
