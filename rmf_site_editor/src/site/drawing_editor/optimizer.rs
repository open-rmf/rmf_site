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

use crate::site::{
    AlignLevelDrawings, Anchor, Angle, Category, ConstraintMarker, Distance, DrawingMarker, Edge,
    FiducialMarker, IsPrimary, LevelProperties, MeasurementMarker, PixelsPerMeter, Point, Pose,
    Rotation, ScaleDrawing,
};
use optimization_engine::{panoc::*, *};
use std::collections::{HashMap, HashSet};

pub fn scale_drawings(
    mut drawings: Query<(&Children, &mut PixelsPerMeter), With<DrawingMarker>>,
    measurements: Query<(&Edge<Entity>, &Distance), With<MeasurementMarker>>,
    anchors: Query<&Anchor>,
    mut events: EventReader<ScaleDrawing>,
) {
    for e in events.iter() {
        if let Ok((children, mut ppm)) = drawings.get_mut(**e) {
            let mut scale_numerator = 0.0;
            let mut scale_denominator = 0;
            for child in children {
                if let Ok((edge, distance)) = measurements.get(*child) {
                    if let Some(in_meters) = distance.0 {
                        let a0 = anchors
                            .get(edge.start())
                            .expect("Broken measurement anchor reference");
                        let d0 = a0.translation_for_category(Category::Drawing);
                        let a1 = anchors
                            .get(edge.end())
                            .expect("Broken measurement anchor reference");
                        let d1 = a1.translation_for_category(Category::Drawing);
                        let in_pixels = ((d0[0] - d1[0]) * (d0[0] - d1[0])
                            + (d0[1] - d1[1]) * (d0[1] - d1[1]))
                            .sqrt();
                        scale_numerator += in_pixels / in_meters;
                        scale_denominator += 1;
                    }
                }
            }
            if scale_denominator > 0 {
                ppm.0 = scale_numerator / (scale_denominator as f32);
            } else {
                println!("No measurements found on current drawing");
            }
        }
    }
}

// The cost will be the sum of the square distances between pairs of points in constraints.
// Cost function is, for each x,y of each edge:
// In matching points, first is reference second is to be optimized
// Order in u is x, y, theta, scale
fn align_level_cost(
    matching_points: &Vec<([f64; 2], [f64; 2])>,
    u: &[f64],
    cost: &mut f64,
) -> Result<(), SolverError> {
    *cost = 0.0;
    let (x, y, theta, s) = (u[0], u[1], u[2], u[3]);
    for (p0, p1) in matching_points {
        *cost += (x + (theta.cos() * p1[0] - theta.sin() * p1[1]) / s - p0[0]).powi(2)
            + (y + (theta.sin() * p1[0] + theta.cos() * p1[1]) / s - p0[1]).powi(2);
    }
    Ok(())
}

fn align_level_gradient(
    matching_points: &Vec<([f64; 2], [f64; 2])>,
    u: &[f64],
    grad: &mut [f64],
) -> Result<(), SolverError> {
    let (x, y, theta, s) = (u[0], u[1], u[2], u[3]);
    grad[0] = 0.0;
    grad[1] = 0.0;
    grad[2] = 0.0;
    grad[3] = 0.0;
    for (p0, p1) in matching_points {
        grad[0] += 2.0 * (x + (theta.cos() * p1[0] - theta.sin() * p1[1]) / s - p0[0]);
        grad[1] += 2.0 * (y + (theta.sin() * p1[0] + theta.cos() * p1[1]) / s - p0[1]);
        // ref https://www.wolframalpha.com/input?i=d%2Fdtheta+%28x+%2B+%28cos%28theta%29+*+p1%5B0%5D+-+sin%28theta%29+*+p1%5B1%5D%29+%2F+s+-+p0%5B0%5D%29%5E2+%2B+%28y+%2B+%28sin%28theta%29+*+p1%5B0%5D+%2B+cos%28theta%29+*+p1%5B1%5D%29+%2F+s+-+p0%5B1%5D%29%5E2
        grad[2] += 2.0 / s
            * (theta.sin() * (p0[1] * p1[1] + p0[0] * p1[0] - p1[0] * x - p1[1] * y)
                + theta.cos() * (p0[0] * p1[1] - p0[1] * p1[0] - p1[1] * x + p1[0] * y));
        // ref https://www.wolframalpha.com/input?i=d%2Fds+%28x+%2B+%28cos%28theta%29+*+p1%5B0%5D+-+sin%28theta%29+*+p1%5B1%5D%29+%2F+s+-+p0%5B0%5D%29%5E2+%2B+%28y+%2B+%28sin%28theta%29+*+p1%5B0%5D+%2B+cos%28theta%29+*+p1%5B1%5D%29+%2F+s+-+p0%5B1%5D%29%5E2
        grad[3] += -2.0
            * (p1[0] * theta.cos() - p1[1] * theta.sin())
            * (-p0[0] + (p1[0] * theta.cos() - p1[1] * theta.sin()) / s + x)
            / (s * s)
            - 2.0
                * (p1[0] * theta.sin() + p1[1] * theta.cos())
                * (-p0[1] + (p1[0] * theta.sin() + p1[1] * theta.cos()) / s + y)
                / (s * s);
    }
    Ok(())
}

pub fn align_level_drawings(
    mut drawings: Query<
        (
            Entity,
            &Children,
            &mut Pose,
            &mut PixelsPerMeter,
            &IsPrimary,
        ),
        With<DrawingMarker>,
    >,
    levels: Query<&Children, With<LevelProperties>>,
    global_tfs: Query<&GlobalTransform>,
    parents: Query<&Parent>,
    anchors: Query<&Anchor>,
    constraints: Query<&Edge<Entity>, With<ConstraintMarker>>,
    mut events: EventReader<AlignLevelDrawings>,
) {
    for e in events.iter() {
        let mut opt_results = HashMap::new();
        // Get the matching points for this entity
        let level_children = levels
            .get(**e)
            .expect("Align level event sent to non level entity");
        let constraints = level_children
            .iter()
            .filter_map(|child| constraints.get(*child).ok())
            .collect::<Vec<_>>();
        if constraints.is_empty() {
            println!("No constraints found for level, skipping optimization");
            continue;
        }
        // TODO(luca) Should we use empty IsPrimary marker instead? Would make it possible to
        // filter drawings by having disjoint queries
        let all_drawings = level_children
            .iter()
            .filter_map(|child| drawings.get(*child).ok());
        let layers = all_drawings
            .clone()
            .filter(|(_, _, _, _, primary)| primary.0 == false)
            .collect::<Vec<_>>();
        if layers.is_empty() {
            println!("No non-primary drawings found for level, at least one drawing must be set to non-primary to be optimized against primary drawings.Skipping optimization");
            continue;
        }
        let references = all_drawings
            .filter(|(_, _, _, _, primary)| primary.0 == true)
            .filter_map(|(e, _, _, _, _)| Some(e))
            .collect::<HashSet<_>>();
        if references.is_empty() {
            println!("No primary drawings found for level. At least one drawing must be set to primary to use as a reference for other drawings. Skipping optimization");
            continue;
        }
        let make_point_pair = |reference: Entity, target: Entity| {
            let reference_point = global_tfs
                .get(reference)
                .expect("Transform for anchor not found")
                .translation()
                .truncate()
                .to_array()
                .map(|t| t as f64);
            let target_point = anchors
                .get(target)
                .expect("Broken constraint anchor reference")
                .translation_for_category(Category::Drawing)
                .map(|t| t as f64);
            (reference_point, target_point)
        };
        for (layer_entity, _, layer_pose, layer_ppm, _) in layers {
            // Optimize this layer
            let mut matching_points = Vec::new();
            let x = layer_pose.trans[0];
            let y = layer_pose.trans[1];
            let theta = match layer_pose.rot.as_yaw() {
                Rotation::Yaw(yaw) => yaw.radians(),
                _ => unreachable!(),
            };
            let s = layer_ppm.0;
            let mut u = vec![x as f64, y as f64, theta as f64, s as f64];
            for edge in constraints.iter() {
                let start_parent = parents
                    .get(edge.start())
                    .expect("Anchor in constraint without drawing parent");
                let end_parent = parents
                    .get(edge.end())
                    .expect("Anchor in constraint without drawing parent");
                if references.contains(&*start_parent) & (layer_entity == **end_parent) {
                    matching_points.push(make_point_pair(edge.start(), edge.end()));
                } else if references.contains(&*end_parent) & (layer_entity == **start_parent) {
                    matching_points.push(make_point_pair(edge.end(), edge.start()));
                } else {
                    println!(
                        "DEV ERROR: Wrong anchors for constraint, must be between primary and non primary drawing"
                    );
                    continue;
                }
            }
            if matching_points.is_empty() {
                println!(
                    "No constraints found for layer {:?}, skipping optimization",
                    layer_entity
                );
                continue;
            }
            let min_vals = vec![
                -std::f64::INFINITY,
                -std::f64::INFINITY,
                -180_f64.to_radians(),
                1e-3,
            ];
            let max_vals = vec![
                std::f64::INFINITY,
                std::f64::INFINITY,
                180_f64.to_radians(),
                1e6,
            ];
            // Now optimize it
            let opt_constraints = constraints::Rectangle::new(Some(&min_vals), Some(&max_vals));
            let mut panoc_cache = PANOCCache::new(u.len(), 1e-6, 10);
            let f = |u: &[f64], c: &mut f64| -> Result<(), SolverError> {
                align_level_cost(&matching_points, u, c)
            };

            let df = |u: &[f64], gradient: &mut [f64]| -> Result<(), SolverError> {
                align_level_gradient(&matching_points, u, gradient)
            };
            let problem = Problem::new(&opt_constraints, df, f);
            let mut panoc = PANOCOptimizer::new(problem, &mut panoc_cache).with_max_iter(1000);
            panoc.solve(&mut u).ok();
            opt_results.insert(layer_entity, (u[0], u[1], u[2], u[3]));
        }
        // Update transform parameters with results of the optimization
        for (e, res) in opt_results.iter() {
            let (_, _, mut pose, mut ppm, _) = drawings.get_mut(*e).unwrap();
            pose.trans[0] = res.0 as f32;
            pose.trans[1] = res.1 as f32;
            pose.rot = Rotation::Yaw(Angle::Rad(res.2 as f32));
            ppm.0 = res.3 as f32;
        }
    }
}
