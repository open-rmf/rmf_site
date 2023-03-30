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
use std::collections::BTreeMap;

use crate::ExportFormat;
use crate::site::Pending;

use thiserror::Error as ThisError;

use rmf_site_format::*;

/// Event used to trigger saving of the workcell
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

fn parent_in_workcell(
    q_parents: &Query<&Parent>,
    entity: Entity,
    root: Entity,
) -> bool {
    AncestorIter::new(q_parents, entity).find(|p| *p == root).is_some()
}

// This is mostly duplicated with the function in site/save.rs, however this case
// is a lot simpler, also site/save.rs checks for children of levels but there are no levels here
fn assign_site_ids(world: &mut World, workcell: Entity) {
    // TODO(luca) actually keep site IDs instead of always generating them from scratch
    // (as it is done in site editor)
    let mut state: SystemState<(
        Query<Entity, (Or<(With<Anchor>, With<WorkcellVisualMarker>)>, Without<Pending>)>,
        Query<&Children>,
    )> = SystemState::new(world);
    let (q_used_entities, q_children) = state.get(&world);

    let mut new_entities = vec!(workcell);
    for e in q_children.iter_descendants(workcell) {
        if let Ok(_) = q_used_entities.get(e) {
            new_entities.push(e);
        }
    }

    for (idx, entity) in new_entities.iter().enumerate() {
        world.entity_mut(*entity).insert(SiteID(idx.try_into().unwrap()));
    }
}

pub fn generate_workcell(
    world: &mut World,
    root: Entity,
) -> Result<rmf_site_format::Workcell, WorkcellGenerationError> {
    assign_site_ids(world, root);
    let mut state: SystemState<(
        Query<(Entity, &Anchor, &SiteID, &Parent, Option<&MeshConstraint<Entity>>), Without<Pending>>,
        Query<
            (
                Entity,
                &NameInSite,
                Option<&AssetSource>,
                Option<&MeshPrimitive>,
                &Pose,
                &SiteID,
                &Parent,
            ),
            (With<WorkcellVisualMarker>, Without<Pending>),
        >,
        Query<&SiteID>,
        Query<&WorkcellProperties>,
        Query<&Parent>,
    )> = SystemState::new(world);
    let (q_anchors, q_models, q_site_id, q_properties, q_parents) = state.get(world);

    let mut workcell = Workcell::default();
    match q_properties.get(root) {
        Ok(properties) => {
            workcell.properties = properties.clone();
        }
        Err(_) => {
            return Err(WorkcellGenerationError::InvalidWorkcellEntity(root));
        }
    }

    // Visuals
    for (e, name, source, primitive, pose, id, parent) in &q_models {
        println!("Found visual");
        if !parent_in_workcell(&q_parents, e, root) {
            continue;
        }
        // Get the parent SiteID
        let parent = match q_site_id.get(parent.get()) {
            Ok(parent) => Some(parent.0),
            Err(_) => None,
        };
        let geom = if let Some(source) = source {
            // It's a model
            Geometry::Mesh{filename: String::from(source)}
        } else if let Some(primitive) = primitive {
            Geometry::Primitive(primitive.clone())
        } else {
            println!("DEV Error, visual without primitive or mesh");
            continue;
        };
        workcell.visuals.insert(
            id.0,
            Parented {
                parent: parent,
                bundle: WorkcellModel {
                    name: name.0.clone(),
                    geometry: geom,
                    pose: pose.clone(),
                },
            },
        );
    }

    // Anchors
    for (e, anchor, id, parent, constraint) in &q_anchors {
        if !parent_in_workcell(&q_parents, e, root) {
            continue;
        }
        let parent = match q_site_id.get(parent.get()) {
            Ok(parent) => Some(parent.0),
            Err(_) => None,
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
                parent: parent,
                bundle: Frame {
                    anchor: anchor.clone(),
                    mesh_constraint: constraint,
                    marker: FrameMarker,
                }
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
        let path = save_event.to_file;
        println!(
            "Saving to {}",
            path.to_str().unwrap_or("<failed to render??>")
        );
        let f = match std::fs::File::create(path) {
            Ok(f) => f,
            Err(err) => {
                println!("Unable to save file: {err}");
                continue;
            }
        };

        let workcell = match generate_workcell(world, save_event.root) {
            Ok(root) => root,
            Err(err) => {
                println!("Unable to compile workcell: {err}");
                continue;
            }
        };

        dbg!(&workcell);

        match save_event.format {
            ExportFormat::Default => {
                match workcell.to_writer(f) {
                    Ok(()) => {
                        println!("Save successful");
                    }
                    Err(err) => {
                        println!("Save failed: {err}");
                    }
                }
            },
            ExportFormat::Urdf => {
                println!("Saving to urdf");
            },
        }

    }
}
