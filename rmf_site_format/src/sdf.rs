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

use crate::{Category, Side, Anchor, DoorType, Level, Pose, Rotation, Site};
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
                        // TODO(luca) set pose
                        vec![
                            SdfJoint {
                                name: "right_joint".into(),
                                parent: "world".into(),
                                child: "right".into(),
                                r#type: "prismatic".into(),
                                axis: Some(SdfJointAxis {
                                    xyz: Vector3d::new(1.0, 0.0, 0.0),
                                    limit: SdfJointaxisLimit {
                                        lower: 0.0,
                                        // TODO(luca) this is length of the door
                                        upper: 0.0,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                                ..Default::default()
                            }
                        ]
                    }
                    DoorType::SingleSwing(swing) => {
                        let (lower, upper) = swing.swing.swing_on_pivot(swing.pivot_on);
                        // TODO(check this logic)
                        let lower = 0.0;
                        let upper = lower - upper.radians() as f64;
                        let pivot = door.anchors.side(swing.pivot_on);
                        let start_anchor = get_anchor(pivot, level)?;
                        let trans = start_anchor.translation_for_category(Category::Level);
                        let end_anchor = get_anchor(door.anchors.side(swing.pivot_on.opposite()), level)?;
                        let end = end_anchor.translation_for_category(Category::Level);
                        let pose = Pose {
                            trans: [trans[0], trans[1], level.properties.elevation.0 + 1.0],
                            ..Default::default()
                        }.to_sdf(0.0);
                        vec![
                            SdfJoint {
                                name: "right_joint".into(),
                                parent: "world".into(),
                                child: "right".into(),
                                r#type: "revolute".into(),
                                axis: Some(SdfJointAxis {
                                    xyz: Vector3d::new(0.0, 0.0, -1.0),
                                    limit: SdfJointaxisLimit {
                                        lower,
                                        upper,
                                        ..Default::default()
                                    },
                                    ..Default::default()
                                }),
                                pose: Some(pose),
                                ..Default::default()
                            }
                        ]
                    }
                    DoorType::DoubleSliding(door) => {
                        vec![
                            SdfJoint {
                                name: "right_joint".into(),
                                parent: "world".into(),
                                child: "right".into(),
                                r#type: "prismatic".into(),
                                ..Default::default()
                            },
                            SdfJoint {
                                name: "left_joint".into(),
                                parent: "world".into(),
                                child: "left".into(),
                                r#type: "prismatic".into(),
                                ..Default::default()
                            },
                        ]
                    }
                    DoorType::DoubleSwing(door) => {
                        vec![
                            SdfJoint {
                                name: "right_joint".into(),
                                parent: "world".into(),
                                child: "right".into(),
                                r#type: "revolute".into(),
                                ..Default::default()
                            },
                            SdfJoint {
                                name: "left_joint".into(),
                                parent: "world".into(),
                                child: "left".into(),
                                r#type: "revolute".into(),
                                ..Default::default()
                            },
                        ]
                    }
                    DoorType::Model(_) => {
                        // Unimplemented! Use a fixed joint for now
                        vec![
                            SdfJoint {
                                name: "right_joint".into(),
                                parent: "world".into(),
                                child: "right".into(),
                                r#type: "fixed".into(),
                                ..Default::default()
                            }
                        ]
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
