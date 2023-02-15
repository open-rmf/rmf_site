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

use crate::site::{DefaultFile, Pending};

use thiserror::Error as ThisError;

use rmf_site_format::{
    Anchor, AssetSource, IsStatic, Model, ModelMarker, NameInSite, Parented, Pose, SiteID, Workcell,
};

/// Event used to trigger saving of the workcell
pub struct SaveWorkcell {
    pub root: Entity,
    pub to_file: Option<PathBuf>,
}

#[derive(ThisError, Debug, Clone)]
pub enum WorkcellGenerationError {
    #[error("the specified entity [{0:?}] does not refer to a workcell")]
    InvalidWorkcellEntity(Entity),
}

// This is mostly duplicated with the function in site/save.rs, however this case
// is a lot simpler, also site/save.rs checks for children of levels but there are no levels here
fn assign_site_ids(world: &mut World, workcell: Entity) {
    // TODO(luca) actually keep site IDs instead of always generating them from scratch
    // (as it is done in site editor)
    let mut state: SystemState<(
        Query<Entity, (Or<(With<Anchor>, With<ModelMarker>)>, Without<Pending>)>,
        Query<&Children>,
    )> = SystemState::new(world);
    let (q_used_entities, q_children) = state.get(&world);

    let mut new_entities = Vec::new();
    for e in q_children.iter_descendants(workcell) {
        if let Ok(_) = q_used_entities.get(e) {
            new_entities.push(e);
        }
    }

    // TODO(luca) cleanup this implementation
    let mut cur_id = 0;
    world.entity_mut(workcell).insert(SiteID(cur_id));
    cur_id += 1;

    for e in new_entities {
        world.entity_mut(e).insert(SiteID(cur_id));
        cur_id += 1;
    }
}

pub fn generate_workcell(
    world: &mut World,
    root: Entity,
) -> Result<rmf_site_format::Workcell, WorkcellGenerationError> {
    assign_site_ids(world, root);
    let mut state: SystemState<(
        Query<(&Anchor, &SiteID, &Parent)>,
        Query<
            (
                &NameInSite,
                &AssetSource,
                &Pose,
                &IsStatic,
                &SiteID,
                &Parent,
            ),
            (With<ModelMarker>, Without<Pending>),
        >,
        Query<&SiteID>,
        Query<&NameInSite>,
    )> = SystemState::new(world);
    let (q_anchors, q_models, q_site_id, q_names) = state.get(world);

    let mut workcell = Workcell::default();
    match q_names.get(root) {
        Ok(workcell_name) => {
            workcell.name = workcell_name.clone();
        }
        Err(_) => {
            return Err(WorkcellGenerationError::InvalidWorkcellEntity(root));
        }
    }

    // Models
    for (name, source, pose, is_static, id, parent) in &q_models {
        println!("Found model {}", name.0);
        // Get the parent SiteID
        let parent_site_id = q_site_id.get(parent.get());
        if let Ok(parent_site_id) = parent_site_id {
            workcell.models.insert(
                id.0,
                Parented {
                    parent: parent_site_id.0,
                    bundle: Model {
                        name: name.clone(),
                        source: source.clone(),
                        pose: pose.clone(),
                        is_static: is_static.clone(),
                        marker: ModelMarker,
                    },
                },
            );
        } else {
            println!(
                "Site ID for entity {:?} not found, skipping...",
                parent.get()
            );
        }
    }

    // Anchors
    for (anchor, id, parent) in &q_anchors {
        println!("Found anchor {:?}", id);
        // Get the parent SiteID
        let parent_site_id = q_site_id.get(parent.get());
        if let Ok(parent_site_id) = parent_site_id {
            workcell.anchors.insert(
                id.0,
                Parented {
                    parent: parent_site_id.0,
                    bundle: anchor.clone(),
                },
            );
        } else {
            println!(
                "Site ID for entity {:?} not found, skipping...",
                parent.get()
            );
        }
    }
    Ok(workcell)
}

pub fn save_workcell(world: &mut World) {
    let save_events: Vec<_> = world
        .resource_mut::<Events<SaveWorkcell>>()
        .drain()
        .collect();
    for save_event in save_events {
        println!("Read workcell save event");
        let path = {
            if let Some(to_file) = save_event.to_file {
                to_file
            } else {
                if let Some(to_file) = world.entity(save_event.root).get::<DefaultFile>() {
                    to_file.0.clone()
                } else {
                    println!("No default save file for workcell, please use [Save As]");
                    continue;
                }
            }
        };

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

        match workcell.to_writer(f) {
            Ok(()) => {
                println!("Save successful");
            }
            Err(err) => {
                println!("Save failed: {err}");
            }
        }
    }
}
