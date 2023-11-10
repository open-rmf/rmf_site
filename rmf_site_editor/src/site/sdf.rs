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
use bevy::render::mesh::shape::{Capsule, UVSphere};

use crate::interaction::Selectable;
use crate::shapes::make_cylinder;
use crate::site::SiteAssets;
use crate::SdfRoot;
use sdformat_rs::{SdfGeometry, SdfPose, Vector3d};

use rmf_site_format::{
    Angle, AssetSource, ConstraintDependents, Geometry, IsStatic, MeshPrimitive, Model,
    ModelMarker, NameInSite, Pose, Rotation, Scale,
};

/// An empty component to mark this entity as a visual mesh
#[derive(Component, Debug, Clone, Default)]
pub struct VisualMeshMarker;

/// An empty component to mark this entity as a collision mesh
#[derive(Component, Debug, Clone, Default)]
pub struct CollisionMeshMarker;

// TODO(luca) cleanup this, there are many ways models are referenced and have to be resolved in
// SDF between local, fuel and cached paths so the logic becomes quite complicated.
fn compute_model_source(path: &str, uri: &str) -> AssetSource {
    let mut asset_source = AssetSource::from(path);
    match asset_source {
        AssetSource::Remote(ref mut p) | AssetSource::Search(ref mut p) => {
            let binding = p.clone();
            *p = if let Some(stripped) = uri.strip_prefix("model://") {
                // Get the org name from context, model name from this and combine
                if let Some(org_name) = binding.split("/").next() {
                    org_name.to_owned() + "/" + stripped
                } else {
                    error!(
                        "Unable to extract organization name from asset source [{}]",
                        uri
                    );
                    "".into()
                }
            } else if let Some(path_idx) = binding.rfind("/") {
                // It's a path relative to this model, remove file and append uri
                let (model_path, _model_name) = binding.split_at(path_idx);
                model_path.to_owned() + "/" + uri
            } else {
                error!(
                    "Invalid SDF model path, Path is [{}] and model uri is [{}]",
                    path, uri
                );
                "".into()
            };
        }
        AssetSource::Local(ref mut p) => {
            let binding = p.clone();
            *p = if let Some(stripped) = uri.strip_prefix("model://") {
                // Search for a model with the requested name in the same folder as the sdf file
                // Note that this will not play well if the requested model shares files with other
                // models that are placed in different folders or are in fuel, but should work for
                // most local, self contained, models.
                // Get the org name from context, model name from this and combine
                if let Some(model_folder) = binding.rsplitn(3, "/").skip(2).next() {
                    model_folder.to_owned() + "/" + stripped
                } else {
                    error!("Unable to extract model folder from asset source [{}]", uri);
                    "".into()
                }
            } else if let Some(path_idx) = binding.rfind("/") {
                // It's a path relative to this model, remove file and append uri
                let (model_path, _model_name) = binding.split_at(path_idx);
                model_path.to_owned() + "/" + uri
            } else {
                error!(
                    "Invalid SDF model path, Path is [{}] and model uri is [{}]",
                    path, uri
                );
                "".into()
            };
        }
        AssetSource::Bundled(_) | AssetSource::Package(_) | AssetSource::OSMTile { .. } => {
            warn!("Requested asset source {:?} type not supported for SDFs, might behave unexpectedly", asset_source);
        }
    }
    asset_source
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

fn spawn_geometry(
    commands: &mut Commands,
    geometry: &SdfGeometry,
    visual_name: &str,
    pose: &Option<SdfPose>,
    sdf_path: &str,
    is_static: bool,
) -> Option<Entity> {
    let pose = parse_pose(pose);
    match geometry {
        SdfGeometry::Mesh(mesh) => Some(
            commands
                .spawn(Model {
                    name: NameInSite(visual_name.to_owned()),
                    source: compute_model_source(sdf_path, &mesh.uri),
                    pose,
                    is_static: IsStatic(is_static),
                    scale: parse_scale(&mesh.scale),
                    marker: ModelMarker,
                })
                .id(),
        ),
        SdfGeometry::Box(b) => {
            let s = &b.size.0;
            Some(
                commands
                    .spawn(MeshPrimitive::Box {
                        size: [s.x as f32, s.y as f32, s.z as f32],
                    })
                    .insert(pose)
                    .insert(SpatialBundle::INHERITED_IDENTITY)
                    .id(),
            )
        }
        SdfGeometry::Capsule(c) => Some(
            commands
                .spawn(MeshPrimitive::Capsule {
                    radius: c.radius as f32,
                    length: c.length as f32,
                })
                .insert(pose)
                .insert(SpatialBundle::INHERITED_IDENTITY)
                .id(),
        ),
        SdfGeometry::Cylinder(c) => Some(
            commands
                .spawn(MeshPrimitive::Cylinder {
                    radius: c.radius as f32,
                    length: c.length as f32,
                })
                .insert(pose)
                .insert(SpatialBundle::INHERITED_IDENTITY)
                .id(),
        ),
        SdfGeometry::Sphere(s) => Some(
            commands
                .spawn(MeshPrimitive::Sphere {
                    radius: s.radius as f32,
                })
                .insert(pose)
                .insert(SpatialBundle::INHERITED_IDENTITY)
                .id(),
        ),
        _ => None,
    }
}

// TODO(luca) reduce duplication between sdf -> MeshPrimitive and urdf -> MeshPrimitive
pub fn handle_new_sdf_roots(mut commands: Commands, new_sdfs: Query<(Entity, &SdfRoot)>) {
    for (e, sdf) in new_sdfs.iter() {
        for link in &sdf.model.link {
            let link_pose = parse_pose(&link.pose);
            let link_id = commands
                .spawn(SpatialBundle::from_transform(link_pose.transform()))
                .id();
            commands.entity(e).add_child(link_id);
            for visual in &link.visual {
                let id = spawn_geometry(
                    &mut commands,
                    &visual.geometry,
                    &visual.name,
                    &visual.pose,
                    &sdf.path,
                    sdf.model.r#static.unwrap_or(false),
                );
                match id {
                    Some(id) => {
                        commands.entity(id).insert(VisualMeshMarker);
                        commands.entity(link_id).add_child(id);
                    }
                    None => warn!("Found unhandled geometry type {:?}", &visual.geometry),
                }
            }
            for collision in &link.collision {
                let id = spawn_geometry(
                    &mut commands,
                    &collision.geometry,
                    &collision.name,
                    &collision.pose,
                    &sdf.path,
                    sdf.model.r#static.unwrap_or(false),
                );
                match id {
                    Some(id) => {
                        commands.entity(id).insert(CollisionMeshMarker);
                        commands.entity(link_id).add_child(id);
                    }
                    None => warn!("Found unhandled geometry type {:?}", &collision.geometry),
                }
            }
        }
        commands.entity(e).remove::<SdfRoot>();
    }
}

pub fn handle_new_mesh_primitives(
    mut commands: Commands,
    primitives: Query<(Entity, &MeshPrimitive), Added<MeshPrimitive>>,
    parents: Query<&Parent>,
    selectables: Query<
        &Selectable,
        Or<(
            With<ModelMarker>,
            With<VisualMeshMarker>,
            With<CollisionMeshMarker>,
        )>,
    >,
    mut meshes: ResMut<Assets<Mesh>>,
    site_assets: Res<SiteAssets>,
) {
    for (e, primitive) in primitives.iter() {
        let mesh = match primitive {
            MeshPrimitive::Box { size } => Mesh::from(shape::Box::new(size[0], size[1], size[2])),
            MeshPrimitive::Cylinder { radius, length } => {
                Mesh::from(make_cylinder(*length, *radius))
            }
            MeshPrimitive::Capsule { radius, length } => Mesh::from(Capsule {
                radius: *radius,
                depth: *length,
                ..default()
            }),
            MeshPrimitive::Sphere { radius } => Mesh::from(UVSphere {
                radius: *radius,
                ..default()
            }),
        };
        // Parent is the first of ModelMarker and / or WorkcellVisualMarker or
        // WorkcelLCollisionMarker
        let child_id = commands
            .spawn(PbrBundle {
                mesh: meshes.add(mesh),
                material: site_assets.default_mesh_grey_material.clone(),
                ..default()
            })
            .id();
        if let Some(selectable) = AncestorIter::new(&parents, e)
            .filter_map(|p| selectables.get(p).ok())
            .last()
        {
            commands.entity(child_id).insert(selectable.clone());
        }
        commands.entity(e).push_children(&[child_id]);
    }
}
