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

use crate::{
    Anchor, Angle, AssetSource, Category, DoorType, Level, LiftCabin, Pose, Rotation, Site, Swing,
};
use glam::Vec3;
use once_cell::sync::Lazy;
use sdformat_rs::*;
use std::collections::BTreeMap;
use thiserror::Error;
use bevy::{prelude::*, utils::tracing};
use tracing::error;

const DEFAULT_CABIN_MASS: f64 = 1200.0;

static WORLD_TEMPLATE: Lazy<SdfRoot> = Lazy::new(|| {
    match yaserde::de::from_str(include_str!("templates/gz_world.sdf")) {
        Ok(sdf_root) => sdf_root,
        Err(err) => {
            error!("Failed deserializing template {:?}", err);
            Default::default()
        }
    }
});

#[derive(Debug, Error)]
pub enum SdfConversionError {
    #[error("An asset that can't be converted to an sdf world was found")]
    UnsupportedAssetType,
    #[error("Entity [{0}] referenced a non existing anchor")]
    BrokenAnchorReference(u32),
    #[error("Entity [{0}] referenced a non existing level")]
    BrokenLevelReference(u32),
    #[error("Parsing lift cabin for lift [{0}] failed")]
    LiftParsingError(String),
    #[error("Lift [{0}] had no initial level where it could be spawned")]
    MissingInitialLevel(String),
    #[error("Unable to find any scenarios")]
    UnableToFindScenario,
    #[error("Entity [{0}] referenced a non existing model instance")]
    BrokenModelInstanceReference(u32),
    #[error("Entity [{0}] referenced a non existing model description")]
    BrokenModelDescriptionReference(u32),
}

impl Pose {
    fn to_sdf(&self) -> SdfPose {
        let p = &self.trans;
        let r = match self.rot {
            Rotation::Yaw(angle) => format!("0 0 {}", angle.radians()),
            Rotation::EulerExtrinsicXYZ(rpy) => format!(
                "{} {} {}",
                rpy[0].radians(),
                rpy[1].radians(),
                rpy[2].radians()
            ),
            Rotation::Quat(quat) => format!("{} {} {} {}", quat[3], quat[0], quat[1], quat[2]),
        };
        SdfPose {
            data: format!("{} {} {} {}", p[0], p[1], p[2], r),
            ..Default::default()
        }
    }
}

fn make_sdf_door_link(mesh_prefix: &str, link_name: &str) -> SdfLink {
    SdfLink {
        name: link_name.to_string(),
        collision: vec![SdfCollision {
            name: format!("{link_name}_collision"),
            geometry: SdfGeometry::Mesh(SdfMeshShape {
                uri: format!("meshes/{mesh_prefix}_{link_name}.glb"),
                ..Default::default()
            }),
            surface: Some(SdfSurface {
                contact: Some(SdfSurfaceContact {
                    collide_bitmask: Some("0x02".into()),
                    ..Default::default()
                }),
                ..Default::default()
            }),
            ..Default::default()
        }],
        visual: vec![SdfVisual {
            name: format!("{link_name}_visual"),
            geometry: SdfGeometry::Mesh(SdfMeshShape {
                uri: format!("meshes/{mesh_prefix}_{link_name}.glb"),
                ..Default::default()
            }),
            ..Default::default()
        }],
        ..Default::default()
    }
}

fn make_sdf_door(
    left_anchor: Anchor,
    right_anchor: Anchor,
    offset: Vec3,
    ros_interface: bool,
    kind: &DoorType,
    mesh_prefix: &str,
    model_name: &str,
) -> Result<SdfModel, SdfConversionError> {
    let left_trans = left_anchor.translation_for_category(Category::Door);
    let right_trans = right_anchor.translation_for_category(Category::Door);
    let center = [
        (left_trans[0] + right_trans[0]) / 2.0,
        (left_trans[1] + right_trans[1]) / 2.0,
    ];
    let dx = left_trans[0] - right_trans[0];
    let dy = left_trans[1] - right_trans[1];
    let door_length = (dx * dx + dy * dy).sqrt();
    let yaw = -dx.atan2(dy);
    let labels = match kind {
        DoorType::SingleSliding(_) | DoorType::SingleSwing(_) | DoorType::Model(_) => {
            Vec::from(["body"])
        }
        DoorType::DoubleSliding(_) | DoorType::DoubleSwing(_) => Vec::from(["right", "left"]),
    };
    let mut plugin = SdfPlugin {
        name: "register_component".into(),
        filename: "libregister_component.so".into(),
        ..Default::default()
    };
    let mut component = XmlElement {
        name: "component".into(),
        ..Default::default()
    };
    let mut door_plugin_inner = XmlElement {
        name: "door".into(),
        ..Default::default()
    };
    component.attributes.insert("name".into(), "Door".into());
    let mut component_data = ElementMap::default();
    door_plugin_inner
        .attributes
        .insert("name".to_string(), model_name.to_owned());
    let mut door_model = SdfModel {
        name: model_name.to_owned(),
        pose: Some(
            Pose {
                trans: (Vec3::from([center[0], center[1], 0.0]) + offset).to_array(),
                rot: Rotation::Yaw(Angle::Rad(yaw)),
            }
            .to_sdf(),
        ),
        r#static: Some(false),
        ..Default::default()
    };
    for label in labels.iter() {
        door_model.link.push(make_sdf_door_link(mesh_prefix, label));
    }
    let mut door_motion_params = vec![];
    let joints = match kind {
        DoorType::SingleSliding(door) => {
            door_plugin_inner
                .attributes
                .insert("type".into(), "SlidingDoor".into());
            door_plugin_inner
                .attributes
                .insert("left_joint_name".into(), "empty_joint".into());
            door_plugin_inner
                .attributes
                .insert("right_joint_name".into(), model_name.to_owned() + "_joint");
            door_motion_params.push(("v_max_door", "0.2"));
            door_motion_params.push(("a_max_door", "0.2"));
            door_motion_params.push(("a_nom_door", "0.08"));
            door_motion_params.push(("dx_min_door", "0.001"));
            door_motion_params.push(("f_max_door", "100.0"));
            let pose = Pose {
                trans: [0.0, (door_length / 2.0) * door.towards.sign(), 1.25],
                ..Default::default()
            }
            .to_sdf();
            vec![SdfJoint {
                name: model_name.to_owned() + "_joint",
                parent: "world".into(),
                child: "body".into(),
                r#type: "prismatic".into(),
                pose: Some(pose),
                axis: Some(SdfJointAxis {
                    xyz: Vector3d::new(0.0, door.towards.sign().into(), 0.0),
                    limit: SdfJointAxisLimit {
                        lower: 0.0,
                        upper: door_length as f64,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }]
        }
        DoorType::SingleSwing(door) => {
            door_plugin_inner
                .attributes
                .insert("type".into(), "SwingDoor".into());
            door_motion_params.push(("v_max_door", "0.5"));
            door_motion_params.push(("a_max_door", "0.3"));
            door_motion_params.push(("a_nom_door", "0.15"));
            door_motion_params.push(("dx_min_door", "0.01"));
            door_motion_params.push(("f_max_door", "500.0"));
            let side = door.pivot_on.sign() as f64;
            let (open, z) = match door.swing {
                Swing::Forward(angle) => (angle.radians() as f64, side),
                Swing::Backward(angle) => (angle.radians() as f64, -side),
                // Only use the forward position for double doors
                Swing::Both { forward, .. } => (forward.radians() as f64, side),
            };
            let lower = 0.0;
            let upper = open.abs();
            let pose = Pose {
                trans: [0.0, (door_length / 2.0) * door.pivot_on.sign(), 1.25],
                ..Default::default()
            }
            .to_sdf();
            let (left_joint_name, right_joint_name) =
                ("empty_joint", model_name.to_owned() + "_joint");
            door_plugin_inner
                .attributes
                .insert("left_joint_name".into(), left_joint_name.into());
            door_plugin_inner
                .attributes
                .insert("right_joint_name".into(), right_joint_name);
            vec![SdfJoint {
                name: model_name.to_owned() + "_joint",
                parent: "world".into(),
                child: "body".into(),
                r#type: "revolute".into(),
                axis: Some(SdfJointAxis {
                    xyz: Vector3d::new(0.0, 0.0, z),
                    limit: SdfJointAxisLimit {
                        lower,
                        upper,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                pose: Some(pose),
                ..Default::default()
            }]
        }
        DoorType::DoubleSliding(door) => {
            door_plugin_inner
                .attributes
                .insert("type".into(), "DoubleSlidingDoor".into());
            door_plugin_inner.attributes.insert(
                "left_joint_name".into(),
                model_name.to_owned() + "_left_joint",
            );
            door_plugin_inner.attributes.insert(
                "right_joint_name".into(),
                model_name.to_owned() + "_right_joint",
            );
            door_motion_params.push(("v_max_door", "0.2"));
            door_motion_params.push(("a_max_door", "0.2"));
            door_motion_params.push(("a_nom_door", "0.08"));
            door_motion_params.push(("dx_min_door", "0.001"));
            door_motion_params.push(("f_max_door", "100.0"));
            let right_pose = Pose {
                trans: [0.0, -door_length / 2.0, 1.25],
                ..Default::default()
            }
            .to_sdf();
            let left_pose = Pose {
                trans: [0.0, door_length / 2.0, 1.25],
                ..Default::default()
            }
            .to_sdf();
            let left_length = (door.left_right_ratio / (1.0 + door.left_right_ratio)) * door_length;
            let right_length = door_length - left_length;
            vec![
                SdfJoint {
                    name: model_name.to_owned() + "_right_joint",
                    parent: "world".into(),
                    child: "right".into(),
                    r#type: "prismatic".into(),
                    pose: Some(right_pose),
                    axis: Some(SdfJointAxis {
                        xyz: Vector3d::new(0.0, -1.0, 0.0),
                        limit: SdfJointAxisLimit {
                            lower: 0.0,
                            upper: right_length as f64,
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    ..Default::default()
                },
                SdfJoint {
                    name: model_name.to_owned() + "_left_joint",
                    parent: "world".into(),
                    child: "left".into(),
                    r#type: "prismatic".into(),
                    pose: Some(left_pose),
                    axis: Some(SdfJointAxis {
                        xyz: Vector3d::new(0.0, -1.0, 0.0),
                        limit: SdfJointAxisLimit {
                            lower: -left_length as f64,
                            upper: 0.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    ..Default::default()
                },
            ]
        }
        DoorType::DoubleSwing(door) => {
            door_plugin_inner
                .attributes
                .insert("type".into(), "DoubleSwingDoor".into());
            door_plugin_inner.attributes.insert(
                "left_joint_name".into(),
                model_name.to_owned() + "_left_joint",
            );
            door_plugin_inner.attributes.insert(
                "right_joint_name".into(),
                model_name.to_owned() + "_right_joint",
            );
            door_motion_params.push(("v_max_door", "0.5"));
            door_motion_params.push(("a_max_door", "0.3"));
            door_motion_params.push(("a_nom_door", "0.15"));
            door_motion_params.push(("dx_min_door", "0.01"));
            door_motion_params.push(("f_max_door", "500.0"));
            let (open, z) = match door.swing {
                Swing::Forward(angle) => (angle.radians() as f64, -1.0),
                Swing::Backward(angle) => (angle.radians() as f64, 1.0),
                // Only use the forward position for double doors
                Swing::Both { forward, .. } => (forward.radians() as f64, -1.0),
            };
            let upper = open.abs();
            let right_pose = Pose {
                trans: [0.0, -door_length / 2.0, 1.25],
                ..Default::default()
            }
            .to_sdf();
            let left_pose = Pose {
                trans: [0.0, door_length / 2.0, 1.25],
                ..Default::default()
            }
            .to_sdf();
            vec![
                SdfJoint {
                    name: model_name.to_owned() + "_right_joint",
                    parent: "world".into(),
                    child: "right".into(),
                    r#type: "revolute".into(),
                    axis: Some(SdfJointAxis {
                        xyz: Vector3d::new(0.0, 0.0, z),
                        limit: SdfJointAxisLimit {
                            lower: 0.0,
                            upper,
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    pose: Some(right_pose),
                    ..Default::default()
                },
                SdfJoint {
                    name: model_name.to_owned() + "_left_joint",
                    parent: "world".into(),
                    child: "left".into(),
                    r#type: "revolute".into(),
                    axis: Some(SdfJointAxis {
                        xyz: Vector3d::new(0.0, 0.0, z),
                        limit: SdfJointAxisLimit {
                            lower: -upper,
                            upper: 0.0,
                            ..Default::default()
                        },
                        ..Default::default()
                    }),
                    pose: Some(left_pose),
                    ..Default::default()
                },
            ]
        }
        DoorType::Model(_) => {
            // Unimplemented! Use a fixed joint for now
            let pose = Pose {
                trans: [0.0, door_length / 2.0, 1.25],
                ..Default::default()
            }
            .to_sdf();
            vec![SdfJoint {
                name: model_name.to_owned() + "_joint",
                parent: "world".into(),
                child: "body".into(),
                r#type: "fixed".into(),
                pose: Some(pose),
                ..Default::default()
            }]
        }
    };
    let b = ros_interface.to_string();
    door_motion_params.push(("ros_interface", &b));
    door_model.joint.extend(joints);
    for (name, value) in door_motion_params.into_iter() {
        component_data.push(XmlElement {
            name: name.into(),
            data: ElementData::String(value.to_string()),
            ..Default::default()
        });
    }
    component_data.push(door_plugin_inner);
    component.data = ElementData::Nested(component_data);
    plugin.elements.push(component);
    door_model.plugin = vec![plugin];
    Ok(door_model)
}

impl Site {
    pub fn to_sdf(&self) -> Result<SdfRoot, SdfConversionError> {
        let get_anchor = |id: u32| -> Result<Anchor, SdfConversionError> {
            self.get_anchor(id)
                .ok_or(SdfConversionError::BrokenAnchorReference(id))
                .cloned()
        };
        let get_level = |id: u32| -> Result<&Level, SdfConversionError> {
            self.levels
                .get(&id)
                .ok_or(SdfConversionError::BrokenLevelReference(id))
        };
        let mut root = WORLD_TEMPLATE.clone();
        let world = &mut root.world[0];
        let mut min_elevation = f32::MAX;
        let mut max_elevation = f32::MIN;
        let mut toggle_floors_plugin = SdfPlugin {
            name: "toggle_floors".into(),
            filename: "toggle_floors".into(),
            ..Default::default()
        };
        // Only export default scenario into SDF for now
        let (_, default_scenario) = self
            .scenarios
            .first_key_value()
            .ok_or(SdfConversionError::UnableToFindScenario)?;
        for (level_id, level) in &self.levels {
            let mut level_model_names = vec![];
            let mut model_element_map = ElementMap::default();
            max_elevation = max_elevation.max(level.properties.elevation.0);
            min_elevation = min_elevation.min(level.properties.elevation.0);
            let mut floor_models_ele = XmlElement {
                name: "floor".into(),
                ..Default::default()
            };
            let level_model_name = &level.properties.name.0;
            floor_models_ele
                .attributes
                .insert("name".into(), level.properties.name.0.clone());
            floor_models_ele
                .attributes
                .insert("model_name".into(), level_model_name.clone());
            // Floors walls and static models are included in the level mesh
            world.model.push(SdfModel {
                name: level_model_name.clone(),
                r#static: Some(true),
                link: vec![SdfLink {
                    name: "link".into(),
                    collision: vec![SdfCollision {
                        name: "collision".into(),
                        geometry: SdfGeometry::Mesh(SdfMeshShape {
                            uri: format!("meshes/level_{}_collision.glb", level_id),
                            ..Default::default()
                        }),
                        surface: Some(SdfSurface {
                            contact: Some(SdfSurfaceContact {
                                collide_bitmask: Some("0x01".into()),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    visual: vec![SdfVisual {
                        name: "visual".into(),
                        geometry: SdfGeometry::Mesh(SdfMeshShape {
                            uri: format!("meshes/level_{}_visual.glb", level_id),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            });
            // TODO(luca) We need this because there is no concept of ingestor or dispenser in
            // rmf_site yet. Remove when there is
            for (model_instance_id, _) in &default_scenario.instances {
                let parented_model_instance = self.model_instances.get(model_instance_id).ok_or(
                    SdfConversionError::BrokenModelInstanceReference(*model_instance_id),
                )?;
                let Some(model_description_id) = parented_model_instance.bundle.description.0
                else {
                    continue;
                };
                let model_description_bundle =
                    self.model_descriptions.get(&model_description_id).ok_or(
                        SdfConversionError::BrokenModelDescriptionReference(*model_instance_id),
                    )?;

                let mut added = false;
                if model_description_bundle.source.0
                    == AssetSource::Search("OpenRobotics/TeleportIngestor".to_string())
                {
                    world.include.push(SdfWorldInclude {
                        uri: "model://TeleportIngestor".to_string(),
                        name: Some(parented_model_instance.bundle.name.0.clone()),
                        pose: Some(parented_model_instance.bundle.pose.to_sdf()),
                        ..Default::default()
                    });
                    added = true;
                } else if model_description_bundle.source.0
                    == AssetSource::Search("OpenRobotics/TeleportDispenser".to_string())
                {
                    world.include.push(SdfWorldInclude {
                        uri: "model://TeleportDispenser".to_string(),
                        name: Some(parented_model_instance.bundle.name.0.clone()),
                        pose: Some(parented_model_instance.bundle.pose.to_sdf()),
                        ..Default::default()
                    });
                    added = true;
                }
                // Non static models are included separately and are not part of the static world
                // TODO(luca) this will duplicate multiple instances of the model since it uses
                // NameInSite instead of AssetSource for the URI, fix
                else if !model_description_bundle.is_static.0 .0 {
                    world.model.push(SdfModel {
                        name: parented_model_instance.bundle.name.0.clone(),
                        r#static: Some(model_description_bundle.is_static.0 .0),
                        pose: Some(parented_model_instance.bundle.pose.to_sdf()),
                        link: vec![SdfLink {
                            name: "link".into(),
                            collision: vec![SdfCollision {
                                name: "collision".into(),
                                geometry: SdfGeometry::Mesh(SdfMeshShape {
                                    uri: format!(
                                        "meshes/model_{}_collision.glb",
                                        model_description_id
                                    ),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }],
                            visual: vec![SdfVisual {
                                name: "visual".into(),
                                geometry: SdfGeometry::Mesh(SdfMeshShape {
                                    uri: format!(
                                        "meshes/model_{}_visual.glb",
                                        model_description_id
                                    ),
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }],
                            ..Default::default()
                        }],
                        ..Default::default()
                    });
                    added = true;
                }
                if added {
                    level_model_names.push(model_description_bundle.name.0.clone());
                }
            }
            // Now add all the doors
            for (door_id, door) in &level.doors {
                let left_anchor = get_anchor(door.anchors.left())?;
                let right_anchor = get_anchor(door.anchors.right())?;
                let door_model = make_sdf_door(
                    left_anchor,
                    right_anchor,
                    Vec3::new(0.0, 0.0, level.properties.elevation.0),
                    true,
                    &door.kind,
                    format!("door_{door_id}").as_str(),
                    door.name.0.as_str(),
                )?;
                level_model_names.push(door_model.name.clone());
                world.model.push(door_model);
            }
            for model_name in level_model_names.into_iter() {
                let model_element = XmlElement {
                    name: "model".into(),
                    attributes: [("name".into(), model_name.into())].into(),
                    ..Default::default()
                };
                model_element_map.push(model_element);
            }
            floor_models_ele.data = ElementData::Nested(model_element_map);
            toggle_floors_plugin.elements.push(floor_models_ele);
        }
        for (lift_id, lift) in &self.lifts {
            let get_lift_anchor = |id: u32| -> Result<Anchor, SdfConversionError> {
                lift.cabin_anchors
                    .get(&id)
                    .ok_or(SdfConversionError::BrokenAnchorReference(id))
                    .cloned()
            };
            // Cabin
            let LiftCabin::Rect(ref cabin) = lift.properties.cabin;
            let pose = lift
                .properties
                .center(self)
                .ok_or(SdfConversionError::LiftParsingError(
                    lift.properties.name.0.clone(),
                ))?;
            let mut plugin = SdfPlugin {
                name: "register_component".into(),
                filename: "libregister_component.so".into(),
                ..Default::default()
            };
            let mut component = XmlElement {
                name: "component".into(),
                ..Default::default()
            };
            component.attributes.insert("name".into(), "Lift".into());
            let mut component_data = ElementMap::default();
            let mut elements = vec![];
            let lift_name = &lift.properties.name.0;
            elements.push(("lift_name", lift_name.clone()));
            let initial_floor = lift
                .properties
                .initial_level
                .0
                .and_then(|id| self.levels.get(&id))
                .map(|level| level.properties.name.0.clone())
                .ok_or(SdfConversionError::MissingInitialLevel(lift_name.clone()))?;
            elements.push(("initial_floor", initial_floor));
            elements.push(("v_max_cabin", "2.0".to_string()));
            elements.push(("a_max_cabin", "1.2".to_string()));
            elements.push(("a_nom_cabin", "1.0".to_string()));
            elements.push(("dx_min_cabin", "0.001".to_string()));
            elements.push(("f_max_cabin", "25323.0".to_string()));
            elements.push(("cabin_joint_name", "cabin_joint".to_string()));
            let mut levels: BTreeMap<u32, ElementMap> = BTreeMap::new();
            let mut lift_models = Vec::new();
            let mut lift_joints = vec![SdfJoint {
                name: "cabin_joint".into(),
                r#type: "prismatic".into(),
                parent: "world".into(),
                child: "platform".into(),
                axis: Some(SdfJointAxis {
                    xyz: Vector3d::new(0.0, 0.0, 1.0),
                    limit: SdfJointAxisLimit {
                        lower: -std::f64::INFINITY,
                        upper: std::f64::INFINITY,
                        ..Default::default()
                    },
                    ..Default::default()
                }),
                ..Default::default()
            }];
            for (face, door_placement) in cabin.doors().iter() {
                let Some(door_placement) = door_placement else {
                    continue;
                };
                // TODO(luca) use door struct for offset / shift
                // TODO(luca) remove unwrap
                let door = lift.cabin_doors.get(&door_placement.door).unwrap();
                let cabin_door_name = format!("CabinDoor_{}_door_{}", lift_name, face.label());
                let cabin_mesh_prefix = format!("lift_{}_{}", lift_id, face.label());
                let left_anchor = get_lift_anchor(door.reference_anchors.left())?;
                let right_anchor = get_lift_anchor(door.reference_anchors.right())?;
                let x_offset = -face.u()
                    * (door_placement.thickness() / 2.0
                        + door_placement
                            .custom_gap
                            .unwrap_or_else(|| cabin.gap.unwrap_or(0.01)));
                let mut cabin_door = make_sdf_door(
                    left_anchor,
                    right_anchor,
                    x_offset,
                    false,
                    &door.kind,
                    &cabin_mesh_prefix,
                    &cabin_door_name,
                )?;
                for mut joint in cabin_door.joint.drain(..) {
                    // Move the joint to the lift and change its parenthood accordingly
                    joint.parent = "platform".into();
                    joint.child = format!("{}::{}", cabin_door.name, joint.child);
                    lift_joints.push(joint);
                }
                lift_models.push(cabin_door.into());
                for visit in door.visits.0.iter() {
                    let level = get_level(*visit)?;
                    let shaft_door_name = format!(
                        "ShaftDoor_{}_{}_door_{}",
                        level.properties.name.0,
                        lift_name,
                        face.label()
                    );
                    let left_anchor = get_lift_anchor(door.reference_anchors.left())?;
                    let right_anchor = get_lift_anchor(door.reference_anchors.right())?;
                    let shaft_door = make_sdf_door(
                        left_anchor,
                        right_anchor,
                        Vec3::from(pose.trans) + Vec3::new(0.0, 0.0, level.properties.elevation.0),
                        false,
                        &door.kind,
                        &cabin_mesh_prefix,
                        &shaft_door_name,
                    )?;
                    // Add the pose of the lift to have world coordinates
                    world.model.push(shaft_door);
                    // Add the shaft door to the level transparency plugin
                    toggle_floors_plugin.elements.for_each_mut("floor", |elem| {
                        if elem.attributes.get("name") == Some(&level.properties.name.0) {
                            if let ElementData::Nested(ref mut map) = elem.data {
                                map.push(XmlElement {
                                    name: "model".into(),
                                    attributes: [("name".into(), shaft_door_name.clone())].into(),
                                    ..Default::default()
                                });
                            }
                        }
                    });
                    let level = levels.entry(*visit).or_default();
                    let element = XmlElement {
                        name: "door_pair".into(),
                        attributes: [
                            ("cabin_door".to_string(), cabin_door_name.clone()),
                            ("shaft_door".to_string(), shaft_door_name),
                        ]
                        .into(),
                        ..Default::default()
                    };
                    level.push(element);
                }
            }
            for (key, door_pairs) in levels.into_iter() {
                let level = get_level(key)?;
                component_data.push(XmlElement {
                    name: "floor".into(),
                    attributes: [
                        ("name".to_string(), level.properties.name.0.clone()),
                        (
                            "elevation".to_string(),
                            level.properties.elevation.0.to_string(),
                        ),
                    ]
                    .into(),
                    data: ElementData::Nested(door_pairs),
                    ..Default::default()
                });
            }
            for (name, value) in elements.into_iter() {
                component_data.push(XmlElement {
                    name: name.into(),
                    data: ElementData::String(value),
                    ..Default::default()
                });
            }
            component.data = ElementData::Nested(component_data);
            plugin.elements.push(component);
            world.model.push(SdfModel {
                name: lift.properties.name.0.clone(),
                r#static: Some(lift.properties.is_static.0),
                pose: Some(pose.to_sdf()),
                link: vec![SdfLink {
                    name: "platform".into(),
                    collision: vec![SdfCollision {
                        name: "collision".into(),
                        geometry: SdfGeometry::Mesh(SdfMeshShape {
                            uri: format!("meshes/lift_{}.glb", lift_id),
                            ..Default::default()
                        }),
                        surface: Some(SdfSurface {
                            contact: Some(SdfSurfaceContact {
                                collide_bitmask: Some("0x04".into()),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    visual: vec![SdfVisual {
                        name: "visual".into(),
                        geometry: SdfGeometry::Mesh(SdfMeshShape {
                            uri: format!("meshes/lift_{}.glb", lift_id),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    inertial: Some(SdfInertial {
                        mass: Some(DEFAULT_CABIN_MASS),
                        inertia: Some(lift.properties.cabin.moment_of_inertia(DEFAULT_CABIN_MASS)),
                        ..Default::default()
                    }),
                    ..Default::default()
                }],
                joint: lift_joints,
                model: lift_models,
                plugin: vec![plugin],
                ..Default::default()
            });
            // TODO(luca) lifts in the legacy pipeline seem to also have a "ramp" joint to allow
            // easier transition of robots into lifts.
            // From tests simulation seems to also work without it, probably due to having changed
            // from full joint with wheel torques to just kinematic simulation for whole robots.
        }

        world.name = self.properties.name.0.clone();
        if let Some(gui) = world.gui.as_mut() {
            gui.plugin.push(toggle_floors_plugin);
            if let Some(minimal_scene) = gui
                .plugin
                .iter_mut()
                .find(|plugin| plugin.filename == "MinimalScene")
            {
                if let Some(camera_pose) = minimal_scene.elements.get_mut("camera_pose") {
                    if let Some(user_camera_pose) = self
                        .levels
                        .first_key_value()
                        .and_then(|(_, level)| level.user_camera_poses.values().next())
                    {
                        // TODO(luca) use level elevation here? It also seems that quaternion
                        // notation in Gazebo and Bevy is different, check
                        let mut pose = user_camera_pose.pose.clone();
                        pose.rot = Rotation::EulerExtrinsicXYZ([
                            Angle::Rad(0.0),
                            Angle::Rad(0.6),
                            Angle::Rad(1.57),
                        ]);
                        pose.trans[0] = pose.trans[0] + 10.0;
                        camera_pose.data = ElementData::String(pose.to_sdf().data);
                    }
                }
            }
        }
        // TODO(luca) these fields are set as required in the specification but seem not to be in
        // practice (rightly so because not everyone wants to manually specify gravity, earth
        // magnetic field and atmosphere model)
        world.atmosphere = SdfAtmosphere {
            r#type: "adiabatic".to_string(),
            ..Default::default()
        };
        world.gravity = Vector3d::new(0.0, 0.0, -9.80);
        world.magnetic_field = Vector3d::new(5.64e-6, 2.29e-5, -4.24e-5);
        Ok(root)
    }
}

#[cfg(test)]
mod tests {
    use bevy::utils::tracing::error;
    use crate::legacy::building_map::BuildingMap;

    #[test]
    fn serialize_sdf() {
        let data = std::fs::read("../assets/demo_maps/office.building.yaml").unwrap();
        let map = BuildingMap::from_bytes(&data).unwrap();
        let site = map.to_site().unwrap();
        // Convert to an sdf
        let sdf = site.to_sdf().unwrap();
        let config = yaserde::ser::Config {
            perform_indent: true,
            write_document_declaration: true,
            ..Default::default()
        };
        let s = yaserde::ser::to_string_with_config(&sdf, &config).unwrap();
        if let Err(e) = std::fs::write("test.sdf", s) {
            error!("Unable to write test.sdf {:?}", e);
        };
    }
}
