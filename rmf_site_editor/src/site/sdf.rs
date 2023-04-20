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

fn compute_model_source(path: &str, uri: &str) -> AssetSource {
    if let Some(stripped) = uri.strip_prefix("model://") {
        // Get the org name from context, model name from this and combine
        let binding = path.strip_prefix("search://").unwrap();
        let mut tokens = binding.split("/");
        if let Some(org_name) = tokens.next() {
            let path = org_name.to_owned() + "/" + stripped;
            return AssetSource::Remote(path);
        }
    } else {
        println!("Non model path found, not spawning! {}", uri);
    }
    // TODO handle other paths?
    AssetSource::Remote(String::new())
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
                                scale: Scale::default(),
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
