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

use bevy::prelude::*;

use crate::{SdfGeometry, SdfRoot};

use rmf_site_format::{
    AssetSource, ConstraintDependents, IsStatic, Model, ModelMarker, NameInSite, Pose, Scale,
};

// TODO(luca) reduce chances for panic and do proper error handling here
fn compute_model_source(path: &str, uri: &str) -> AssetSource {
    let binding = path.strip_prefix("search://").unwrap();
    if let Some(stripped) = uri.strip_prefix("model://") {
        // Get the org name from context, model name from this and combine
        let org_name = binding.split("/").next().unwrap();
        let path = org_name.to_owned() + "/" + stripped;
        AssetSource::Remote(path)
    } else if let Some(path_idx) = binding.rfind("/") {
        // It's a path relative to this model, remove file and append uri
        let (model_path, _model_name) = binding.split_at(path_idx);
        AssetSource::Remote(model_path.to_owned() + "/" + uri)
    } else {
        AssetSource::Remote("".into())
    }
}

fn parse_scale(scale: &Option<String>) -> Scale {
    match scale {
        Some(v) => {
            let split_results: Vec<_> = v
                .split_whitespace()
                .filter_map(|s| s.parse::<f32>().ok())
                .collect();
            if split_results.len() != 3 {
                return Scale::default();
            }
            let mut res = [0.0f32; 3];
            res.copy_from_slice(&split_results);
            Scale(Vec3::from_slice(&res))
        }
        None => Scale::default(),
    }
}

pub fn handle_new_sdf_roots(mut commands: Commands, new_sdfs: Query<(Entity, &SdfRoot)>) {
    for (e, sdf) in new_sdfs.iter() {
        for link in &sdf.model.link {
            for visual in &link.visual {
                match &visual.geometry {
                    SdfGeometry::Mesh(mesh) => {
                        let id = commands
                            .spawn(Model {
                                name: NameInSite("Unnamed".to_string()),
                                source: compute_model_source(&sdf.path, &mesh.uri),
                                pose: Pose::default(),
                                is_static: IsStatic::default(),
                                constraints: ConstraintDependents::default(),
                                scale: parse_scale(&mesh.scale),
                                marker: ModelMarker,
                            })
                            .id();
                        commands.entity(e).add_child(id);
                    }
                }
            }
            /*
            for collision in &link.collision {
                let model = WorkcellModel::from(collision);
                let cmd =
                    commands.spawn((SpatialBundle::VISIBLE_IDENTITY, WorkcellCollisionMarker));
                let id = cmd.id();
                model.add_bevy_components(cmd);
                commands.entity(link_entity).add_child(id);
            }
            */
        }
        commands.entity(e).remove::<SdfRoot>();
    }
}
