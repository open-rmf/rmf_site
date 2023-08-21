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
    ModelMarker, NameInSite, Pose, Rotation, Scale, WorkcellCollisionMarker, WorkcellVisualMarker,
};

/// An empty component to mark this entity as a visual mesh
#[derive(Component, Debug, Clone, Default)]
pub struct VisualMeshMarker;

/// An empty component to mark this entity as a collision mesh
#[derive(Component, Debug, Clone, Default)]
pub struct CollisionMeshMarker;

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
                    constraints: ConstraintDependents::default(),
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
                    .insert(SpatialBundle::VISIBLE_IDENTITY)
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
                .insert(SpatialBundle::VISIBLE_IDENTITY)
                .id(),
        ),
        SdfGeometry::Cylinder(c) => Some(
            commands
                .spawn(MeshPrimitive::Cylinder {
                    radius: c.radius as f32,
                    length: c.length as f32,
                })
                .insert(pose)
                .insert(SpatialBundle::VISIBLE_IDENTITY)
                .id(),
        ),
        SdfGeometry::Sphere(s) => Some(
            commands
                .spawn(MeshPrimitive::Sphere {
                    radius: s.radius as f32,
                })
                .insert(pose)
                .insert(SpatialBundle::VISIBLE_IDENTITY)
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
            With<WorkcellVisualMarker>,
            With<WorkcellCollisionMarker>,
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
