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
use std::collections::{HashMap, HashSet};

use crate::interaction::Selectable;
use crate::site::{AnchorBundle, SiteAssets};
use crate::shapes::{make_box, make_cylinder};

use rmf_site_format::{Anchor, Angle, AssetSource, Category, Geometry, Link, MeshPrimitive, Model, NameInSite, NameInWorkcell, Pose, Rotation, UrdfRoot, WorkcellModel, WorkcellCollisionMarker, WorkcellVisualMarker};

use urdf_rs::{JointType};

use bevy_rapier3d::prelude::*;
use bevy_rapier3d::rapier::math::{Isometry};
use bevy_rapier3d::na::geometry::{Rotation as RapierRotation};

pub fn handle_new_urdf_roots(
    mut commands: Commands,
    new_urdfs: Query<(Entity, &UrdfRoot)>,
    site_assets: Res<SiteAssets>,
) {
    let mut link_name_to_entity = HashMap::new();
    // Keep track of which links have a parent, add the ones that don't as a root child
    let mut root_links = HashSet::new();
    for (e, urdf) in new_urdfs.iter() {
        commands.entity(e)
            .insert(RigidBody::KinematicVelocityBased);
        // Populate here
        for link in &urdf.links {
            // TODO*luca) link as child of anchor
            let link_entity = commands.spawn(SpatialBundle::VISIBLE_IDENTITY)
                .insert(Link::from(link))
                .insert(RigidBody::KinematicVelocityBased)
                .insert(Category::Workcell)
                .id();
            println!("Found link {:?} - {}", link_entity, link.name);
            link_name_to_entity.insert(link.name.clone(), link_entity);
            root_links.insert(link_entity);
            for visual in &link.visual {
                let model = WorkcellModel::from(visual);
                let mut cmd = commands.spawn((SpatialBundle::VISIBLE_IDENTITY, WorkcellVisualMarker));
                let id = cmd.id();
                model.add_bevy_components(cmd);
                commands.entity(link_entity).add_child(id);
            }
            for collision in &link.collision {
                let model = WorkcellModel::from(collision);
                let mut cmd = commands.spawn((SpatialBundle::VISIBLE_IDENTITY, WorkcellCollisionMarker));
                let id = cmd.id();
                model.add_bevy_components(cmd);
                commands.entity(link_entity).add_child(id);
            }
        }
        for joint in &urdf.joints {
            if let Some(parent) = link_name_to_entity.get(&joint.parent.link) {
                if let Some(child) = link_name_to_entity.get(&joint.child.link) {
                    let trans = Vec3::from_array(joint.origin.xyz.map(|t| t as f32));
                    let rot = Vec3::from_array(joint.origin.rpy.map(|t| t as f32));
                    let rot = RapierRotation::from_euler_angles(rot[0], rot[1], rot[2]);
                    // TODO(luca) invert the above since it's in joint coordinates
                    let frame = Isometry::<f32>::from_parts(trans.into(), rot.into());
                    let joint_data = match joint.joint_type {
                        JointType::Revolute => {
                            let axis = Vec3::from_array(joint.axis.xyz.map(|t| t as f32));
                            let joint = RevoluteJointBuilder::new(axis)
                                //.local_anchor2(trans)
                                .limits([joint.limit.lower as f32, joint.limit.upper as f32]);
                            ImpulseJoint::new(*parent, joint)
                        },
                        JointType::Prismatic => {
                            let axis = Vec3::from_array(joint.axis.xyz.map(|t| t as f32));
                            let joint = PrismaticJointBuilder::new(axis)
                                //.local_anchor2(trans)
                                .local_axis2(axis)
                                .limits([joint.limit.lower as f32, joint.limit.upper as f32]);
                            ImpulseJoint::new(*parent, joint)
                        },
                        JointType::Fixed => {
                            let joint = FixedJointBuilder::new()
                                .local_anchor1(trans)
                                //.local_anchor2(trans)
                                //.local_basis2(rot.into())
                                ;
                            ImpulseJoint::new(*parent, joint)
                        },
                        _ => {todo!("Unimplemented joint type {:?}", joint.joint_type);}
                    };
                    let trans = joint.origin.xyz.map(|t| t as f32);
                    let mut rot = Rotation::EulerExtrinsicXYZ(joint.origin.rpy.map(|angle| Angle::Rad(angle as f32)));
                    //commands.entity(*child).insert(AnchorBundle::new(Anchor::Pose3D(Pose {trans, rot})));
                    commands.entity(*parent).add_child(*child);
                    root_links.remove(child);
                    println!("Adding joint between {:?} - {} and {:?} - {}", *parent, &joint.parent.link, *child, &joint.child.link);
                    commands.entity(*child).with_children(|children| {children.spawn(SpatialBundle::VISIBLE_IDENTITY).insert(joint_data).insert(Anchor::Pose3D(Pose {trans, rot}));});
                }
            }
        }
        for link in root_links.iter() {
            println!("Found root entity {:?}", link);
            commands.entity(e).add_child(*link);
        }
        commands.entity(e).remove::<UrdfRoot>();
    }
}

pub fn handle_new_mesh_primitives(
    mut commands: Commands,
    primitives: Query<(Entity, &MeshPrimitive), Added<MeshPrimitive>>,
    mut meshes: ResMut<Assets<Mesh>>,
    site_assets: Res<SiteAssets>,
) {
    for (e, primitive) in primitives.iter() {
        let mesh = match primitive {
            MeshPrimitive::Box{size} => {Mesh::from(make_box(size[0], size[1], size[2]))}
            MeshPrimitive::Cylinder{radius, length} => {Mesh::from(make_cylinder(*length, *radius))}
            MeshPrimitive::Capsule{radius, length} => {Mesh::from(Capsule{radius: *radius, depth: *length, ..default()})}
            MeshPrimitive::Sphere{radius} => {Mesh::from(UVSphere{radius: *radius, ..default()})}
        };
        let child_id = commands.spawn(PbrBundle {
            mesh: meshes.add(mesh),
            material: site_assets.default_mesh_grey_material.clone(),
            ..default()})
            .insert(Selectable::new(e))
            .id();
        commands.entity(e).push_children(&[child_id]);
    }
}
