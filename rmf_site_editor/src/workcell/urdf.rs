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

use crate::UrdfRoot;
use crate::site::{AnchorBundle, SiteAssets};
use crate::shapes::{make_box, make_cylinder};

use rmf_site_format::{Anchor, Angle, AssetSource, Category, Link, MeshPrimitive, Model, NameInSite, Pose, Rotation};

use urdf_rs::{JointType, Geometry};

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
            .insert(RigidBody::KinematicVelocityBased)
            /*
            .insert(Velocity {
                linvel: Vec3::new(1.0, 2.0, 3.0),
                angvel: Vec3::new(0.2, 0.0, 0.0),
            })
            */
        ;
        //dbg!(urdf);
        // Populate here
        let mut ctr = 0;
        for link in &urdf.links {
            // TODO*luca) link as child of anchor
            let link_entity = commands.spawn_empty()
                .insert(SpatialBundle::VISIBLE_IDENTITY)
                .insert(Link::new(link.name.clone()))
                // TODO(luca) actual collision mesh here
                //.insert(Collider::ball(0.000001))
                .insert(RigidBody::KinematicVelocityBased)
                .insert(Category::Workcell)
                .id();
            println!("Found link {:?} - {}", link_entity, link.name);
            link_name_to_entity.insert(link.name.clone(), link_entity);
            root_links.insert(link_entity);
            for visual in &link.visual {
                let trans = visual.origin.xyz.map(|t| t as f32);
                let rot = Rotation::EulerExtrinsicXYZ(visual.origin.rpy.map(|t| Angle::Rad(t as f32)));
                dbg!(&visual.geometry);
                let visual_child = match &visual.geometry {
                    Geometry::Box{size} => {
                        commands.spawn(MeshPrimitive::Box{size: size.map(|s| s as f32)})
                            .insert(SpatialBundle::VISIBLE_IDENTITY).id()
                    },
                    Geometry::Cylinder{radius, length} => {
                        commands.spawn(MeshPrimitive::Cylinder{radius: *radius as f32, length: *length as f32})
                            .insert(SpatialBundle::VISIBLE_IDENTITY).id()

                    },
                    Geometry::Capsule{radius, length} => {
                        commands.spawn(MeshPrimitive::Capsule{radius: *radius as f32, length: *length as f32})
                            .insert(SpatialBundle::VISIBLE_IDENTITY).id()
                    },
                    Geometry::Sphere{radius} => {
                        commands.spawn(MeshPrimitive::Sphere{radius: *radius as f32})
                            .insert(SpatialBundle::VISIBLE_IDENTITY).id()
                    },
                    Geometry::Mesh{filename, scale} => {
                        // TODO(luca) implement scale
                        let source = AssetSource::from(filename);
                        commands.spawn(Model {
                            // TODO(luca) NameInWorkcell?
                            name: NameInSite(visual.name.clone().unwrap_or("Unnamed".to_string())),
                            source: source,
                            pose: Pose{trans, rot},
                            ..default()
                        }).id()
                    },
                };
                commands.entity(link_entity).push_children(&[visual_child]);
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
                            MultibodyJoint::new(*parent, joint)
                        },
                        JointType::Prismatic => {
                            let axis = Vec3::from_array(joint.axis.xyz.map(|t| t as f32));
                            let joint = PrismaticJointBuilder::new(axis)
                                //.local_anchor2(trans)
                                .local_axis2(axis)
                                .limits([joint.limit.lower as f32, joint.limit.upper as f32]);
                            MultibodyJoint::new(*parent, joint)
                        },
                        JointType::Fixed => {
                            let joint = FixedJointBuilder::new()
                                .local_anchor1(trans)
                                //.local_basis2(rot.into())
                                ;
                            MultibodyJoint::new(*parent, joint)
                        },
                        _ => {todo!("Unimplemented joint type {:?}", joint.joint_type);}
                    };
                    let trans = joint.origin.xyz.map(|t| t as f32);
                    let mut rot = Rotation::EulerExtrinsicXYZ(joint.origin.rpy.map(|angle| Angle::Rad(angle as f32)));
                    commands.entity(*child).insert(AnchorBundle::new(Anchor::Pose3D(Pose {trans, rot})));
                    commands.entity(*parent).push_children(&[*child]);
                    root_links.remove(child);
                    println!("Adding joint between {:?} - {} and {:?} - {}", *parent, &joint.parent.link, *child, &joint.child.link);
                    commands.entity(*child).with_children(|children| {children.spawn(joint_data);});
                }
            }
        }
        for link in root_links.iter() {
            println!("Found root entity {:?}", link);
            commands.entity(e).push_children(&[*link]);
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
        dbg!(&primitive);
        let child_id = commands.spawn(PbrBundle {
            mesh: meshes.add(mesh),
            material: site_assets.default_mesh_grey_material.clone(),
            ..default()
        }).id();
        commands.entity(e).push_children(&[child_id]);
    }
}
