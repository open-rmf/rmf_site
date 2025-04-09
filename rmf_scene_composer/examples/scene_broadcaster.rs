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
use glam::{EulerRot, Quat};
use rmf_scene_composer::gz::msgs::{
    BoxGeom, CapsuleGeom, Color, CylinderGeom, Geometry, Light, Link, MeshGeom, Model, Pose,
    Quaternion, Scene, SphereGeom, Vector3d, Visual, geometry, light::LightType, visual,
};
use sdformat_rs::{SdfGeometry, SdfLight, SdfLink, SdfModel, SdfPose, SdfRoot};

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

fn main() {
    let args = Args::parse();

    let data = std::fs::read_to_string(args.file).unwrap();
    let root = sdformat_rs::from_str::<SdfRoot>(&data).unwrap();
    println!("Original sdf:\n{root:#?}");
    let proto = convert_sdf_to_proto(root);
    println!("Proto:\n{proto:#?}");
}

fn convert_sdf_to_proto(sdf: SdfRoot) -> Scene {
    let mut scene_models = Vec::<Model>::new();
    let mut scene_lights = Vec::<Light>::new();

    if let Some(model) = sdf.model {
        let mut models = parse_model(&model);
        scene_models.append(&mut models);
    }
    if let Some(light) = sdf.light {
        scene_lights.push(parse_light(&light));
    }

    for world in sdf.world {
        // Add models and lights in SdfWorld to Scene directly
        for world_model in world.model {
            let mut models = parse_model(&world_model);
            scene_models.append(&mut models);
        }
        for world_light in world.light {
            scene_lights.push(parse_light(&world_light));
        }
    }

    // We only need model and light data for generate_scene
    Scene {
        model: Vec::new(),
        light: Vec::new(),
        ..Default::default()
    }
}

fn parse_pose(pose: &Option<SdfPose>) -> Option<Pose> {
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

    if rotation_format == "quat_xyzw" && position.len() == 7 {
        return Some(Pose {
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
        });
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
            euler_angles[0],
            euler_angles[1],
            euler_angles[2],
        );
        return Some(Pose {
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
        });
    }

    None
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

fn parse_light(light: &SdfLight) -> Light {
    Light {
        r#type: if light.r#type == "spot".to_string() {
            LightType::Spot.into()
        } else if light.r#type == "directional".to_string() {
            LightType::Directional.into()
        } else {
            LightType::Point.into() // Default for SdfLight is "point"
        },
        pose: parse_pose(&light.pose),
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

fn parse_link(link: &SdfLink) -> Link {
    Link {
        pose: parse_pose(&link.pose),
        visual: link
            .visual
            .iter()
            .map(|v| Visual {
                cast_shadows: v.cast_shadows.unwrap_or_default(),
                transparency: v.transparency.unwrap_or_default(),
                pose: parse_pose(&v.pose),
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
            .map(|l| parse_light(&l))
            .collect::<Vec<Light>>(),
        ..Default::default()
    }
}

fn parse_model(model: &SdfModel) -> Vec<Model> {
    let mut model_vec = Vec::<Model>::new();
    let mut model_links = model
        .link
        .iter()
        .map(|l| parse_link(&l))
        .collect::<Vec<Link>>();
    let model_pose = parse_pose(&model.pose);

    for included_model in &model.include {
        let link = Link {
            visual: vec![Visual {
                geometry: Some(Geometry {
                    r#type: geometry::Type::Mesh as i32,
                    mesh: Some(MeshGeom {
                        filename: included_model.uri.clone(),
                        ..Default::default()
                    }),
                    ..Default::default()
                }),
                ..Default::default()
            }],
            ..Default::default()
        };
        if included_model.merge.is_some_and(|merge| merge) {
            // Merge nested model into the top model - flatten included model's links
            model_links.push(link);
        } else {
            // Add included model to the vec as its own entity, with pose relative to top model
            model_vec.push(Model {
                is_static: included_model.r#static.unwrap_or_default(),
                pose: add_pose(model_pose.clone(), parse_pose(&included_model.pose)),
                link: vec![link],
                ..Default::default()
            });
        }
    }

    // Push the top model
    model_vec.push(Model {
        is_static: model.r#static.unwrap_or(false),
        pose: model_pose,
        link: model_links,
        model: model.model.iter().fold(Vec::<Model>::new(), |mut v, m| {
            let inner_model: &SdfModel = &*m;
            let mut models = parse_model(inner_model);
            v.append(&mut models);
            v
        }),
        ..Default::default()
    });

    model_vec
}

fn add_pose(pose_a: Option<Pose>, pose_b: Option<Pose>) -> Option<Pose> {
    let Some((position_a, quat_a)) = pose_a.and_then(|a| a.position.zip(a.orientation)) else {
        return None;
    };
    let Some((position_b, quat_b)) = pose_b.and_then(|b| b.position.zip(b.orientation)) else {
        return None;
    };

    Some(Pose {
        position: Some(Vector3d {
            x: position_a.x + position_b.x,
            y: position_a.y + position_b.y,
            z: position_a.z + position_b.z,
            ..Default::default()
        }),
        orientation: Some(Quaternion {
            x: quat_a.x + quat_b.x,
            y: quat_a.y + quat_b.y,
            z: quat_a.z + quat_b.z,
            w: quat_a.w + quat_b.w,
            ..Default::default()
        }),
        ..Default::default()
    })
}
