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
        AnchorBundle, CollisionMeshMarker, DefaultFile, Dependents, ModelLoadingRequest,
        ModelSpawningExt, PreventDeletion, VisualMeshMarker,
    },
    workcell::ChangeCurrentWorkcell,
    WorkspaceMarker,
};
use bevy::prelude::*;
use std::collections::HashSet;

use rmf_site_format::{
    Category, ConstraintDependents, FrameMarker, Geometry, IsStatic, MeshConstraint, Model,
    ModelMarker, NameInSite, NameInWorkcell, Parented, Scale, SiteID, Workcell, WorkcellModel,
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

// Helper type only used in this module to help with visual / collision spawning
enum WorkcellModelType {
    Visual,
    Collision,
}

fn generate_workcell_entities(commands: &mut Commands, workcell: &Workcell) -> Entity {
    // Create hashmap of ids to entity to correctly generate hierarchy
    let mut id_to_entity = HashMap::new();
    // Hashmap of parent id to list of its children entities
    let mut parent_to_child_entities = HashMap::new();
    // Hashmap of parent model entity to constraint dependent entity
    let mut model_to_constraint_dependent_entities = HashMap::new();

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

    let mut add_model = |parented: &Parented<u32, WorkcellModel>,
                         id: u32,
                         commands: &mut Commands,
                         model_type: WorkcellModelType|
     -> Entity {
        let e = match &parented.bundle.geometry {
            Geometry::Primitive(primitive) => {
                let mut cmd = commands.spawn((
                    primitive.clone(),
                    parented.bundle.pose.clone(),
                    NameInWorkcell(parented.bundle.name.clone()),
                ));
                match model_type {
                    WorkcellModelType::Visual => cmd.insert((VisualMeshMarker, Category::Visual)),
                    WorkcellModelType::Collision => {
                        cmd.insert((CollisionMeshMarker, Category::Collision))
                    }
                };
                cmd.id()
            }
            Geometry::Mesh { source, scale } => {
                let scale = Scale(scale.unwrap_or(Vec3::ONE));
                let pose = parented.bundle.pose.clone();
                let name = NameInWorkcell(parented.bundle.name.clone());
                let id = commands.spawn(ConstraintDependents::default()).id();
                let req = match model_type {
                    WorkcellModelType::Visual => ModelLoadingRequest::new(id, source.clone())
                        .then_command(move |cmd: &mut Commands| {
                            cmd.entity(id).insert((
                                name,
                                pose,
                                scale,
                                IsStatic::default(),
                                ModelMarker,
                                VisualMeshMarker,
                                Category::Visual,
                            ));
                        }),
                    WorkcellModelType::Collision => ModelLoadingRequest::new(id, source.clone())
                        .then_command(move |cmd: &mut Commands| {
                            cmd.entity(id).insert((
                                name,
                                pose,
                                scale,
                                IsStatic::default(),
                                ModelMarker,
                                CollisionMeshMarker,
                                Category::Collision,
                            ));
                        }),
                };
                commands.spawn_model(req);
                id
            }
        };
        commands.entity(e).insert(SiteID(id));
        let child_entities: &mut Vec<Entity> =
            parent_to_child_entities.entry(parented.parent).or_default();
        child_entities.push(e);
        id_to_entity.insert(id, e);
        e
    };

    for (id, parented_visual) in &workcell.visuals {
        add_model(parented_visual, *id, commands, WorkcellModelType::Visual);
    }

    for (id, parented_collision) in &workcell.collisions {
        add_model(
            parented_collision,
            *id,
            commands,
            WorkcellModelType::Collision,
        );
    }

    for (id, parented_anchor) in &workcell.frames {
        let e = commands
            .spawn(AnchorBundle::new(parented_anchor.bundle.anchor.clone()).visible(true))
            .insert(SiteID(*id))
            .insert(FrameMarker)
            .id();
        if let Some(c) = &parented_anchor.bundle.mesh_constraint {
            let model_entity = *id_to_entity
                .get(&c.entity)
                .expect("Mesh constraint refers to non existing model");
            commands.entity(e).insert(MeshConstraint {
                entity: model_entity,
                element: c.element.clone(),
                relative_pose: c.relative_pose,
            });
            let constraint_dependents: &mut HashSet<Entity> =
                model_to_constraint_dependent_entities
                    .entry(model_entity)
                    .or_default();
            constraint_dependents.insert(e);
        }
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

    // Add constraint dependents to models
    for (model, dependents) in model_to_constraint_dependent_entities {
        commands
            .entity(model)
            .insert(ConstraintDependents(dependents));
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
) {
    for cmd in load_workcells.read() {
        info!("Loading workcell");
        let root = generate_workcell_entities(&mut commands, &cmd.workcell);
        if let Some(path) = &cmd.default_file {
            commands.entity(root).insert(DefaultFile(path.clone()));
        }

        if cmd.focus {
            change_current_workcell.send(ChangeCurrentWorkcell { root });
        }
    }
}
