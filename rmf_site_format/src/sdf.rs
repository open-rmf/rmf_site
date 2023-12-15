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

use crate::{Anchor, Angle, Category, DoorType, Level, Pose, Rotation, Side, Site, Swing};
use sdformat_rs::*;

#[derive(Debug)]
pub enum SdfConversionError {
    /// An asset that can't be converted to an sdf world was found.
    UnsupportedAssetType,
    /// Entity referenced a non existing anchor.
    BrokenAnchorReference(u32),
}

impl Pose {
    fn to_sdf(&self, elevation: f32) -> SdfPose {
        let p = &self.trans;
        let r = match self.rot {
            Rotation::Yaw(angle) => format!("0 0 {}", angle.radians()),
            Rotation::EulerExtrinsicXYZ(rpy) => format!(
                "{} {} {}",
                rpy[0].radians(),
                rpy[1].radians(),
                rpy[2].radians()
            ),
            Rotation::Quat(quat) => format!("{} {} {} {}", quat[0], quat[1], quat[2], quat[3]),
        };
        SdfPose {
            data: format!("{} {} {} {}", p[0], p[1], p[2] + elevation, r),
            ..Default::default()
        }
    }
}

impl Site {
    pub fn to_sdf(&self) -> Result<SdfRoot, SdfConversionError> {
        let get_anchor = |id: u32, level: &Level| -> Result<Anchor, SdfConversionError> {
            level
                .anchors
                .get(&id)
                .or_else(|| self.anchors.get(&id))
                .ok_or(SdfConversionError::BrokenAnchorReference(id))
                .cloned()
        };
        let door_mass = 50.0;
        let mut models = Vec::new();
        for level in self.levels.values() {
            // Floors walls and static models are included in the level mesh
            models.push(SdfModel {
                name: format!("level_{}", level.properties.name.0),
                r#static: Some(true),
                link: vec![SdfLink {
                    name: "link".into(),
                    collision: vec![SdfCollision {
                        name: "collision".into(),
                        geometry: SdfGeometry::Mesh(SdfMeshShape {
                            uri: format!("meshes/level_{}_collision.glb", level.properties.name.0),
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
                            uri: format!("meshes/level_{}_visual.glb", level.properties.name.0),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }],
                    ..Default::default()
                }],
                ..Default::default()
            });
            // Now add all the doors
            for door in level.doors.values() {
                let left_anchor = get_anchor(door.anchors.left(), level)?;
                let left_trans = left_anchor.translation_for_category(Category::Door);
                let right_anchor = get_anchor(door.anchors.right(), level)?;
                let right_trans = right_anchor.translation_for_category(Category::Door);
                let center = [
                    (left_trans[0] + right_trans[0]) / 2.0,
                    (left_trans[1] + right_trans[1]) / 2.0,
                ];
                let dx = left_trans[0] - right_trans[0];
                let dy = left_trans[1] - right_trans[1];
                let door_length = (dx * dx + dy * dy).sqrt();
                let yaw = -dx.atan2(dy);
                let labels = match door.kind {
                    DoorType::SingleSliding(_) | DoorType::SingleSwing(_) | DoorType::Model(_) => {
                        Vec::from(["right"])
                    }
                    DoorType::DoubleSliding(_) | DoorType::DoubleSwing(_) => {
                        Vec::from(["right", "left"])
                    }
                };
                let mut door_model = SdfModel {
                    name: door.name.0.clone(),
                    pose: Some(
                        Pose {
                            trans: [center[0], center[1], level.properties.elevation.0],
                            rot: Rotation::Yaw(Angle::Rad(yaw)),
                        }
                        .to_sdf(0.0),
                    ),
                    r#static: Some(false),
                    ..Default::default()
                };
                for label in labels.iter() {
                    door_model.link.push(SdfLink {
                        name: label.to_string(),
                        collision: vec![SdfCollision {
                            name: format!("{}_collision", label),
                            geometry: SdfGeometry::Mesh(SdfMeshShape {
                                uri: format!("meshes/{}_{}.glb", door.name.0, label),
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
                            name: format!("{}_visual", label),
                            geometry: SdfGeometry::Mesh(SdfMeshShape {
                                uri: format!("meshes/{}_{}.glb", door.name.0, label),
                                ..Default::default()
                            }),
                            ..Default::default()
                        }],
                        // TODO(luca) calculate inertia based on door properties
                        inertial: Some(SdfInertial {
                            mass: Some(door_mass),
                            inertia: Some(SdfInertialInertia {
                                ixx: 20.0,
                                iyy: 20.0,
                                izz: 5.0,
                                ..Default::default()
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    });
                }
                let joints = match &door.kind {
                    DoorType::SingleSliding(door) => {
                        let pose = Pose {
                            trans: [0.0, (door_length / 2.0) * door.towards.sign(), 1.25],
                            ..Default::default()
                        }
                        .to_sdf(0.0);
                        vec![SdfJoint {
                            name: "right_joint".into(),
                            parent: "world".into(),
                            child: "right".into(),
                            r#type: "prismatic".into(),
                            pose: Some(pose),
                            axis: Some(SdfJointAxis {
                                xyz: Vector3d::new(0.0, door.towards.sign(), 0.0),
                                limit: SdfJointaxisLimit {
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
                        let side = door.pivot_on.sign() as f64;
                        let closed = 0.0_f64;
                        let (open, z) = match door.swing {
                            Swing::Forward(angle) => (side * angle.radians() as f64, 1.0),
                            Swing::Backward(angle) => (-side * angle.radians() as f64, -1.0),
                            // Only use the forward position for double doors
                            Swing::Both { forward, backward } => {
                                (side * forward.radians() as f64, 1.0)
                            }
                        };
                        let lower = closed.min(closed + open);
                        let upper = closed.max(closed + open);
                        let pose = Pose {
                            trans: [0.0, (door_length / 2.0) * door.pivot_on.sign(), 1.25],
                            ..Default::default()
                        }
                        .to_sdf(0.0);
                        vec![SdfJoint {
                            name: "right_joint".into(),
                            parent: "world".into(),
                            child: "right".into(),
                            r#type: "revolute".into(),
                            axis: Some(SdfJointAxis {
                                xyz: Vector3d::new(0.0, 0.0, z),
                                limit: SdfJointaxisLimit {
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
                        let right_pose = Pose {
                            trans: [0.0, -door_length / 2.0, 1.25],
                            ..Default::default()
                        }
                        .to_sdf(0.0);
                        let left_pose = Pose {
                            trans: [0.0, door_length / 2.0, 1.25],
                            ..Default::default()
                        }
                        .to_sdf(0.0);
                        vec![
                            SdfJoint {
                                name: "right_joint".into(),
                                parent: "world".into(),
                                child: "right".into(),
                                r#type: "prismatic".into(),
                                pose: Some(right_pose),
                                axis: Some(SdfJointAxis {
                                    xyz: Vector3d::new(0.0, 1.0, 0.0),
                                    limit: SdfJointaxisLimit {
                                        lower: 0.0,
                                        upper: door_length as f64 / 2.0,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                                ..Default::default()
                            },
                            SdfJoint {
                                name: "left_joint".into(),
                                parent: "world".into(),
                                child: "left".into(),
                                r#type: "prismatic".into(),
                                pose: Some(left_pose),
                                axis: Some(SdfJointAxis {
                                    xyz: Vector3d::new(0.0, -1.0, 0.0),
                                    limit: SdfJointaxisLimit {
                                        lower: -door_length as f64 / 2.0,
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
                        let closed = 0.0_f64;
                        let (open, z) = match door.swing {
                            Swing::Forward(angle) => (-angle.radians() as f64, 1.0),
                            Swing::Backward(angle) => (angle.radians() as f64, -1.0),
                            // Only use the forward position for double doors
                            Swing::Both { forward, backward } => (-forward.radians() as f64, 1.0),
                        };
                        let lower = closed.min(open);
                        let upper = closed.max(open);
                        let right_pose = Pose {
                            trans: [0.0, -door_length / 2.0, 1.25],
                            ..Default::default()
                        }
                        .to_sdf(0.0);
                        let left_pose = Pose {
                            trans: [0.0, door_length / 2.0, 1.25],
                            ..Default::default()
                        }
                        .to_sdf(0.0);
                        vec![
                            SdfJoint {
                                name: "right_joint".into(),
                                parent: "world".into(),
                                child: "right".into(),
                                r#type: "revolute".into(),
                                axis: Some(SdfJointAxis {
                                    xyz: Vector3d::new(0.0, 0.0, z),
                                    limit: SdfJointaxisLimit {
                                        lower,
                                        upper,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                                pose: Some(right_pose),
                                ..Default::default()
                            },
                            SdfJoint {
                                name: "left_joint".into(),
                                parent: "world".into(),
                                child: "left".into(),
                                r#type: "revolute".into(),
                                axis: Some(SdfJointAxis {
                                    xyz: Vector3d::new(0.0, 0.0, -z),
                                    limit: SdfJointaxisLimit {
                                        lower: -upper,
                                        upper: -lower,
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
                        .to_sdf(0.0);
                        vec![SdfJoint {
                            name: "right_joint".into(),
                            parent: "world".into(),
                            child: "right".into(),
                            r#type: "fixed".into(),
                            pose: Some(pose),
                            ..Default::default()
                        }]
                    }
                };
                door_model.joint.extend(joints);
                // TODO(luca) Plugin element
                models.push(door_model);
            }
        }

        let sun = SdfLight {
            name: "sun".into(),
            r#type: "directional".into(),
            cast_shadows: Some(true),
            diffuse: Some("1 1 1 1".into()),
            pose: Some(Pose::default().to_sdf(10.0)),
            specular: Some("0.2 0.2 0.2 1".into()),
            attenuation: Some(SdfLightAttenuation {
                range: 1000.0,
                constant: Some(0.09),
                linear: Some(0.001),
                quadratic: Some(0.001),
            }),
            direction: Vector3d::new(-0.5, 0.1, -0.9),
            ..Default::default()
        };
        Ok(SdfRoot {
            version: "1.7".to_string(),
            world: vec![SdfWorld {
                name: self.properties.name.0.clone(),
                model: models,
                atmosphere: SdfAtmosphere {
                    r#type: "adiabatic".to_string(),
                    ..Default::default()
                },
                scene: SdfScene {
                    ambient: "1 1 1".to_string(),
                    background: "0.8 0.8 0.8".to_string(),
                    ..Default::default()
                },
                light: vec![sun],
                ..Default::default()
            }],
            ..Default::default()
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::legacy::building_map::BuildingMap;

    #[test]
    fn serde_roundtrip() {
        let data = std::fs::read("../assets/demo_maps/hotel.building.yaml").unwrap();
        let map = BuildingMap::from_bytes(&data).unwrap();
        let site = map.to_site().unwrap();
        // Convert to an sdf
        let sdf = site.to_sdf().unwrap();
        dbg!(&sdf);
        let config = yaserde::ser::Config {
            perform_indent: true,
            write_document_declaration: true,
            ..Default::default()
        };
        let s = yaserde::ser::to_string_with_config(&sdf, &config).unwrap();
        println!("{}", s);
        std::fs::write("test.sdf", s);
        panic!();
    }
}
