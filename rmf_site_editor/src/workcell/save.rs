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

use bevy::ecs::system::SystemState;
use bevy::prelude::*;
use std::path::PathBuf;

use crate::site::{CollisionMeshMarker, Pending, VisualMeshMarker};
use crate::workcell::urdf_package_exporter::{generate_package, PackageContext, Person};
use crate::ExportFormat;

use thiserror::Error as ThisError;

use rmf_site_format::*;

/// Event used to trigger saving of the workcell
#[derive(Event)]
pub struct SaveWorkcell {
    pub root: Entity,
    pub to_file: PathBuf,
    pub format: ExportFormat,
}

#[derive(ThisError, Debug, Clone)]
pub enum WorkcellGenerationError {
    #[error("the specified entity [{0:?}] does not refer to a workcell")]
    InvalidWorkcellEntity(Entity),
}

fn parent_in_workcell(q_parents: &Query<&Parent>, entity: Entity, root: Entity) -> bool {
    AncestorIter::new(q_parents, entity)
        .find(|p| *p == root)
        .is_some()
}

// This is mostly duplicated with the function in site/save.rs, however this case
// is a lot simpler, also site/save.rs checks for children of levels but there are no levels here
fn assign_site_ids(world: &mut World, workcell: Entity) {
    // TODO(luca) actually keep site IDs instead of always generating them from scratch
    // (as it is done in site editor)
    let mut state: SystemState<(
        Query<
            Entity,
            (
                Or<(
                    With<FrameMarker>,
                    With<JointProperties>,
                    With<Moment>,
                    With<VisualMeshMarker>,
                    With<CollisionMeshMarker>,
                )>,
                Without<Pending>,
            ),
        >,
        Query<&Children>,
    )> = SystemState::new(world);
    let (q_used_entities, q_children) = state.get(&world);

    let mut new_entities = vec![workcell];
    for e in q_children.iter_descendants(workcell) {
        if let Ok(_) = q_used_entities.get(e) {
            new_entities.push(e);
        }
    }

    for (idx, entity) in new_entities.iter().enumerate() {
        world
            .entity_mut(*entity)
            .insert(SiteID(idx.try_into().unwrap()));
    }
}

pub fn generate_workcell(
    world: &mut World,
    root: Entity,
) -> Result<rmf_site_format::Workcell, WorkcellGenerationError> {
    assign_site_ids(world, root);
    let mut state: SystemState<(
        Query<
            (
                Entity,
                &Anchor,
                Option<&NameInWorkcell>,
                &SiteID,
                &Parent,
                Option<&MeshConstraint<Entity>>,
            ),
            Without<Pending>,
        >,
        Query<(Entity, &Pose, &Mass, &Moment, &SiteID, &Parent), Without<Pending>>,
        Query<
            (
                Entity,
                &NameInWorkcell,
                Option<&AssetSource>,
                Option<&PrimitiveShape>,
                &Pose,
                &SiteID,
                &Parent,
                Option<&Scale>,
            ),
            (
                Or<(With<VisualMeshMarker>, With<CollisionMeshMarker>)>,
                Without<Pending>,
            ),
        >,
        Query<(Entity, &JointProperties, &NameInWorkcell, &SiteID, &Parent), Without<Pending>>,
        Query<&VisualMeshMarker>,
        Query<&CollisionMeshMarker>,
        Query<&SiteID>,
        Query<&NameOfWorkcell>,
        Query<&Parent>,
    )> = SystemState::new(world);
    let (
        q_anchors,
        q_inertials,
        q_models,
        q_joints,
        q_visuals,
        q_collisions,
        q_site_id,
        q_properties,
        q_parents,
    ) = state.get(world);

    let mut workcell = Workcell::default();
    match q_properties.get(root) {
        Ok(name) => workcell.properties.name = name.clone(),
        Err(_) => {
            return Err(WorkcellGenerationError::InvalidWorkcellEntity(root));
        }
    }

    // Visuals
    for (e, name, source, primitive, pose, id, parent, scale) in &q_models {
        if !parent_in_workcell(&q_parents, e, root) {
            continue;
        }
        // Get the parent SiteID
        let parent = match q_site_id.get(parent.get()) {
            Ok(parent) => parent.0,
            Err(_) => {
                error!("Parent not found for visual {:?}", parent.get());
                continue;
            }
        };
        let geom = if let Some(source) = source {
            // It's a model
            Geometry::Mesh {
                source: source.clone(),
                scale: scale.map(|s| **s),
            }
        } else if let Some(primitive) = primitive {
            Geometry::Primitive(primitive.clone())
        } else {
            error!("DEV Error, visual without primitive or mesh");
            continue;
        };
        if q_visuals.get(e).is_ok() {
            workcell.visuals.insert(
                id.0,
                Parented {
                    parent,
                    bundle: WorkcellModel {
                        name: name.0.clone(),
                        geometry: geom,
                        pose: pose.clone(),
                    },
                },
            );
        } else if q_collisions.get(e).is_ok() {
            // TODO(luca) reduce duplication with above branch
            workcell.collisions.insert(
                id.0,
                Parented {
                    parent,
                    bundle: WorkcellModel {
                        name: name.0.clone(),
                        geometry: geom,
                        pose: pose.clone(),
                    },
                },
            );
        }
    }

    // Anchors
    for (e, anchor, name, id, parent, constraint) in &q_anchors {
        if !parent_in_workcell(&q_parents, e, root) {
            continue;
        }
        let parent = match q_site_id.get(parent.get()) {
            Ok(parent) => parent.0,
            Err(_) => {
                error!("Parent not found for anchor {:?}", parent.get());
                continue;
            }
        };
        // TODO(luca) is duplication here OK? same information is contained in mesh constraint and
        // anchor
        let constraint = if let Some(c) = constraint {
            Some(MeshConstraint {
                entity: **q_site_id.get(c.entity).unwrap(),
                element: c.element.clone(),
                relative_pose: c.relative_pose,
            })
        } else {
            None
        };

        workcell.frames.insert(
            id.0,
            Parented {
                parent,
                bundle: Frame {
                    anchor: anchor.clone(),
                    name: name.cloned(),
                    mesh_constraint: constraint,
                    marker: FrameMarker,
                },
            },
        );
    }

    for (e, pose, mass, moment, id, parent) in &q_inertials {
        if !parent_in_workcell(&q_parents, e, root) {
            continue;
        }
        let parent = match q_site_id.get(parent.get()) {
            Ok(parent) => parent.0,
            Err(_) => {
                error!("Parent not found for inertial {:?}", parent.get());
                continue;
            }
        };

        workcell.inertias.insert(
            id.0,
            Parented {
                parent,
                bundle: Inertia {
                    center: pose.clone(),
                    mass: mass.clone(),
                    moment: moment.clone(),
                },
            },
        );
    }

    for (e, properties, name, id, parent) in &q_joints {
        if !parent_in_workcell(&q_parents, e, root) {
            continue;
        }
        let parent = match q_site_id.get(parent.get()) {
            Ok(parent) => parent.0,
            Err(_) => {
                error!("Parent not found for joint {:?}", parent.get());
                continue;
            }
        };

        workcell.joints.insert(
            id.0,
            Parented {
                parent,
                bundle: Joint {
                    name: name.clone(),
                    properties: properties.clone(),
                },
            },
        );
    }

    Ok(workcell)
}

pub fn save_workcell(world: &mut World) {
    let save_events: Vec<_> = world
        .resource_mut::<Events<SaveWorkcell>>()
        .drain()
        .collect();
    for save_event in save_events {
        let workcell = match generate_workcell(world, save_event.root) {
            Ok(root) => root,
            Err(err) => {
                error!("Unable to compile workcell: {err}");
                continue;
            }
        };

        let path = save_event.to_file;
        match save_event.format {
            ExportFormat::Default => {
                info!(
                    "Saving to {}",
                    path.to_str().unwrap_or("<failed to render??>")
                );
                let f = match std::fs::File::create(path) {
                    Ok(f) => f,
                    Err(err) => {
                        error!("Unable to save file: {err}");
                        continue;
                    }
                };
                match workcell.to_writer(f) {
                    Ok(()) => {
                        info!("Save successful");
                    }
                    Err(err) => {
                        error!("Save failed: {err}");
                    }
                }
            }
            ExportFormat::Urdf => {
                // TODO(luca) File name is ignored, change to pick folder instead of pick file
                match export_package(&path, workcell) {
                    Ok(()) => {
                        info!("Successfully exported package");
                    }
                    Err(err) => {
                        error!("Failed to export package: {err}");
                    }
                };
            }
            ExportFormat::Sdf => {
                warn!("Exporting workcells to sdf is not supported.");
            }
        }
    }
}

fn export_package(path: &PathBuf, workcell: Workcell) -> Result<(), Box<dyn std::error::Error>> {
    let output_directory = path.parent().ok_or("Not able to get parent")?;

    let package_context = PackageContext {
        license: "TODO".to_string(),
        maintainers: vec![Person {
            name: "TODO".to_string(),
            email: "todo@todo.com".to_string(),
        }],
        project_name: workcell.properties.name.0.clone() + "_description",
        fixed_frame: "world".to_string(),
        dependencies: vec![],
        project_description: "TODO".to_string(),
        project_version: "0.0.1".to_string(),
        urdf_file_name: "robot.urdf".to_string(),
    };

    generate_package(workcell, package_context, &output_directory)?;
    Ok(())
}
