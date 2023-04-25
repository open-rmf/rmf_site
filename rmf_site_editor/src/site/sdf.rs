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

use crate::SdfRoot;
use sdformat_rs::{SdfGeometry, SdfPose, Vector3d};

use rmf_site_format::{
    Angle, AssetSource, ConstraintDependents, IsStatic, Model, ModelMarker, NameInSite, Pose,
    Rotation, Scale,
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

fn parse_scale(scale: &Option<Vector3d>) -> Scale {
    match scale {
        Some(v) => Scale(Vec3::new(v.0.x as f32, v.0.y as f32, v.0.z as f32)),
        None => Scale::default(),
    }
}

fn parse_pose(pose: &Option<SdfPose>) -> Pose {
    if let Some(pose) = pose.clone().and_then(|p| p.get_pose().ok()) {
        let rot = pose.rotation.euler_angles();
        Pose {
            trans: [
                pose.translation.x as f32,
                pose.translation.y as f32,
                pose.translation.z as f32,
            ],
            rot: Rotation::EulerExtrinsicXYZ([
                Angle::Rad(rot.0 as f32),
                Angle::Rad(rot.1 as f32),
                Angle::Rad(rot.2 as f32),
            ]),
        }
    } else {
        Pose::default()
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
                                pose: parse_pose(&visual.pose),
                                is_static: IsStatic::default(),
                                constraints: ConstraintDependents::default(),
                                scale: parse_scale(&mesh.scale),
                                marker: ModelMarker,
                            })
                            .id();
                        commands.entity(e).add_child(id);
                    }
                    _ => println!("Found unhandled geometry type {:?}", &visual.geometry),
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
