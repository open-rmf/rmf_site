/*
 * Copyright (C) 2025 Open Source Robotics Foundation
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

use clap::Parser;
use glam::{EulerRot, Quat, Vec3};
use prost::Message;
use rmf_scene_composer::gz::msgs::{
    BoxGeom, CapsuleGeom, Color, CylinderGeom, Geometry, Light, Link, MeshGeom, Model, Pose,
    Quaternion, Scene, SphereGeom, Vector3d, Visual, geometry, light::LightType, visual,
};
use sdformat_rs::{SdfGeometry, SdfLight, SdfLink, SdfModel, SdfPose, SdfRoot};
use std::collections::HashMap;

/// Broadcast the data of an .sdf or .world file as a gz-msgs Scene message
/// over a zenoh topic.
#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
struct Args {
    /// Name of an SDF or world file to publish
    #[arg(short, long)]
    file: String,

    /// Topic name to publish the scene to
    #[arg(short, long)]
    topic: String,
}

#[tokio::main]
async fn main() {
    let args = Args::parse();

    let data = std::fs::read_to_string(args.file).unwrap();
    let root = sdformat_rs::from_str::<SdfRoot>(&data).unwrap();
    let proto = convert_sdf_to_proto(root);

    let session = zenoh::open(zenoh::Config::default()).await.unwrap();
    let publisher = session.declare_publisher(&args.topic).await.unwrap();

    let matching_listener = publisher.matching_listener().await.unwrap();
    println!("Waiting for listener...");
    while let Ok(status) = matching_listener.recv_async().await {
        if status.matching() {
            publisher
                .put(zenoh::bytes::ZBytes::from(proto.encode_to_vec()))
                .await
                .unwrap();
            println!("Done putting");
            return;
        }
    }
}

fn convert_sdf_to_proto(sdf: SdfRoot) -> Scene {
    let mut scene_models = Vec::<Model>::new();
    let mut scene_lights = Vec::<Light>::new();
    let mut scene_poses = HashMap::<String, Pose>::new();

    if let Some(model) = sdf.model {
        scene_models.push(parse_model(&model, &mut scene_poses));
    }
    if let Some(light) = sdf.light {
        scene_lights.push(parse_light(&light, &mut scene_poses));
    }

    for world in sdf.world {
        // Add models and lights in SdfWorld to Scene directly
        for world_model in world.model {
            scene_models.push(parse_model(&world_model, &mut scene_poses));
        }
        for world_light in world.light {
            scene_lights.push(parse_light(&world_light, &mut scene_poses));
        }
    }

    // We only need model and light data for generate_scene
    Scene {
        model: scene_models,
        light: scene_lights,
        ..Default::default()
    }
}

fn parse_pose(pose: &Option<SdfPose>, scene_poses: &HashMap<String, Pose>) -> Option<Pose> {
    let Some(pose) = pose else {
        return None;
    };
    let rotation_format = pose
        .rotation_format
        .clone()
        .unwrap_or("euler_rpy".to_string());
    let position = pose
        .data
        .split_whitespace()
        .filter_map(|s| s.parse::<f64>().ok())
        .collect::<Vec<f64>>();

    let mut scene_pose = if rotation_format == "quat_xyzw" && position.len() == 7 {
        Some(Pose {
            position: Some(Vector3d {
                x: position[0],
                y: position[1],
                z: position[2],
                ..Default::default()
            }),
            orientation: Some(Quaternion {
                x: position[3],
                y: position[4],
                z: position[5],
                w: position[6],
                ..Default::default()
            }),
            ..Default::default()
        })
    } else if rotation_format == "euler_rpy" && position.len() == 6 {
        let euler_angles: Vec<f32> = if pose.degrees.is_some_and(|d| d) {
            // Convert euler angles from degrees to radians
            vec![
                (position[3] * std::f64::consts::PI / 180.0) as f32,
                (position[4] * std::f64::consts::PI / 180.0) as f32,
                (position[5] * std::f64::consts::PI / 180.0) as f32,
            ]
        } else {
            vec![position[3] as f32, position[4] as f32, position[5] as f32]
        };
        let quat = Quat::from_euler(
            EulerRot::ZYX,
            euler_angles[2],
            euler_angles[1],
            euler_angles[0],
        );
        Some(Pose {
            position: Some(Vector3d {
                x: position[0],
                y: position[1],
                z: position[2],
                ..Default::default()
            }),
            orientation: Some(Quaternion {
                x: quat.x as f64,
                y: quat.y as f64,
                z: quat.z as f64,
                w: quat.w as f64,
                ..Default::default()
            }),
            ..Default::default()
        })
    } else {
        None
    };

    // If pose is relative to another Pose in the scene, apply transform
    if let Some(parent_pose) = pose
        .relative_to
        .clone()
        .and_then(|name| scene_poses.get(&name))
    {
        scene_pose = apply_relative_transform(parent_pose, scene_pose);
    }

    scene_pose
}

fn apply_relative_transform(parent_pose: &Pose, transform: Option<Pose>) -> Option<Pose> {
    let position = parent_pose
        .clone()
        .position
        .map(|p| Vec3::new(p.x as f32, p.y as f32, p.z as f32))
        .unwrap_or_default();
    let orientation = parent_pose
        .clone()
        .orientation
        .map(|q| Quat::from_array([q.x as f32, q.y as f32, q.z as f32, q.w as f32]))
        .unwrap_or_default();

    let transformed_pose = transform
        .clone()
        .and_then(|t| {
            t.position
                .map(|p| Vec3::new(p.x as f32, p.y as f32, p.z as f32))
        })
        .map(|translation| position + orientation.mul_vec3(translation))
        .unwrap_or(position.clone());

    let transformed_ori = transform
        .clone()
        .and_then(|t| {
            t.orientation
                .map(|q| Quat::from_array([q.x as f32, q.y as f32, q.z as f32, q.w as f32]))
        })
        .map(|rotation| orientation * rotation)
        .unwrap_or(orientation.clone());

    Some(Pose {
        position: Some(Vector3d {
            x: transformed_pose.x as f64,
            y: transformed_pose.y as f64,
            z: transformed_pose.z as f64,
            ..Default::default()
        }),
        orientation: Some(Quaternion {
            x: transformed_ori.x as f64,
            y: transformed_ori.y as f64,
            z: transformed_ori.z as f64,
            w: transformed_ori.w as f64,
            ..Default::default()
        }),
        ..Default::default()
    })
}

fn parse_light_color(color: &Option<String>) -> Option<Color> {
    if let Some(color) = color {
        let color_vec = color
            .split_whitespace()
            .filter_map(|s| s.parse::<f32>().ok())
            .collect::<Vec<f32>>();
        if color_vec.len() == 4 {
            return Some(Color {
                r: color_vec[0],
                g: color_vec[1],
                b: color_vec[2],
                a: color_vec[3],
                ..Default::default()
            });
        }
    }
    None
}

fn parse_light(light: &SdfLight, scene_poses: &mut HashMap<String, Pose>) -> Light {
    Light {
        r#type: if light.r#type == "spot".to_string() {
            LightType::Spot.into()
        } else if light.r#type == "directional".to_string() {
            LightType::Directional.into()
        } else {
            LightType::Point.into() // Default for SdfLight is "point"
        },
        pose: parse_pose(&light.pose, scene_poses),
        diffuse: parse_light_color(&light.diffuse),
        specular: parse_light_color(&light.specular),
        attenuation_constant: light
            .attenuation
            .clone()
            .and_then(|a| a.constant)
            .unwrap_or(0.0) as f32,
        attenuation_linear: light
            .attenuation
            .clone()
            .and_then(|a| a.linear)
            .unwrap_or(0.0) as f32,
        attenuation_quadratic: light
            .attenuation
            .clone()
            .and_then(|a| a.quadratic)
            .unwrap_or(0.0) as f32,
        direction: Some(Vector3d {
            x: light.direction.0.x,
            y: light.direction.0.y,
            z: light.direction.0.z,
            ..Default::default()
        }),
        range: light
            .attenuation
            .clone()
            .map(|s| s.range)
            .unwrap_or_default() as f32,
        cast_shadows: light.cast_shadows.unwrap_or(true),
        spot_inner_angle: light
            .spot
            .clone()
            .map(|s| s.inner_angle)
            .unwrap_or_default() as f32,
        spot_outer_angle: light
            .spot
            .clone()
            .map(|s| s.outer_angle)
            .unwrap_or_default() as f32,
        spot_falloff: light.spot.clone().map(|s| s.falloff).unwrap_or_default() as f32,
        intensity: light.intensity.unwrap_or_default() as f32,
        is_light_off: light.light_on.unwrap_or(true),
        ..Default::default()
    }
}

fn parse_link(link: &SdfLink, scene_poses: &mut HashMap<String, Pose>) -> Link {
    let link_pose = parse_pose(&link.pose, scene_poses);
    if let Some(pose) = link_pose.clone() {
        scene_poses.insert(link.name.clone(), pose);
    }

    Link {
        pose: link_pose,
        visual: link
            .visual
            .iter()
            .map(|v| Visual {
                cast_shadows: v.cast_shadows.unwrap_or_default(),
                transparency: v.transparency.unwrap_or_default(),
                pose: parse_pose(&v.pose, scene_poses),
                geometry: match &v.geometry {
                    SdfGeometry::Mesh(mesh) => Some(Geometry {
                        r#type: geometry::Type::Mesh as i32,
                        mesh: Some(MeshGeom {
                            filename: mesh.uri.clone(),
                            scale: mesh.scale.clone().map(|s| Vector3d {
                                x: s.0.x,
                                y: s.0.y,
                                z: s.0.z,
                                ..Default::default()
                            }),
                            submesh: mesh
                                .submesh
                                .clone()
                                .map(|s| s.name.clone())
                                .unwrap_or_default(),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    SdfGeometry::Box(box_shape) => Some(Geometry {
                        r#type: geometry::Type::Box as i32,
                        r#box: Some(BoxGeom {
                            size: Some(Vector3d {
                                x: box_shape.size.0.x,
                                y: box_shape.size.0.y,
                                z: box_shape.size.0.z,
                                ..Default::default()
                            }),
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    SdfGeometry::Cylinder(cylinder) => Some(Geometry {
                        r#type: geometry::Type::Cylinder as i32,
                        cylinder: Some(CylinderGeom {
                            radius: cylinder.radius,
                            length: cylinder.length,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    SdfGeometry::Sphere(sphere) => Some(Geometry {
                        r#type: geometry::Type::Sphere as i32,
                        sphere: Some(SphereGeom {
                            radius: sphere.radius,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    SdfGeometry::Capsule(capsule) => Some(Geometry {
                        r#type: geometry::Type::Capsule as i32,
                        capsule: Some(CapsuleGeom {
                            radius: capsule.radius,
                            length: capsule.length,
                            ..Default::default()
                        }),
                        ..Default::default()
                    }),
                    _ => None, // Exclude remaining geometry types currently unsupported by the site editor
                },
                r#type: visual::Type::Link as i32,
                ..Default::default()
            })
            .collect::<Vec<Visual>>(),
        light: link
            .light
            .iter()
            .map(|l| parse_light(&l, scene_poses))
            .collect::<Vec<Light>>(),
        ..Default::default()
    }
}

fn parse_model(model: &SdfModel, scene_poses: &mut HashMap<String, Pose>) -> Model {
    let model_pose = parse_pose(&model.pose, scene_poses);
    if let Some(pose) = model_pose.clone() {
        scene_poses.insert(model.name.clone(), pose);
    }

    Model {
        is_static: model.r#static.unwrap_or(false),
        pose: model_pose,
        link: model
            .link
            .iter()
            .map(|l| parse_link(&l, scene_poses))
            .collect::<Vec<Link>>(),
        model: model
            .model
            .iter()
            .map(|m| {
                let inner_model: &SdfModel = &*m;
                parse_model(inner_model, scene_poses)
            })
            .collect(),
        ..Default::default()
    }
}

fn simple_box_test() -> Scene {
    let mut scene = Scene::default();
    let mut model = rmf_scene_composer::gz::msgs::Model::default();
    let mut link = rmf_scene_composer::gz::msgs::Link::default();
    let mut visual = rmf_scene_composer::gz::msgs::Visual::default();
    let mut geometry = rmf_scene_composer::gz::msgs::Geometry::default();
    let mut cube = rmf_scene_composer::gz::msgs::BoxGeom::default();
    let mut size = rmf_scene_composer::gz::msgs::Vector3d {
        header: None,
        x: 1.0,
        y: 1.0,
        z: 1.0,
    };
    cube.size = Some(size);
    geometry.r#box = Some(cube);
    visual.geometry = Some(geometry);
    link.visual.push(visual);
    model.link.push(link);
    scene.model.push(model);
    scene
}
