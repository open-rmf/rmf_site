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

use bevy::ecs::system::SystemParam;
use bevy::prelude::*;

use crate::site::{
    AlignLevelDrawings, AlignSiteDrawings, Anchor, Angle, Category, Change, ConstraintMarker,
    Distance, DrawingMarker, Edge, LevelElevation, MeasurementMarker, PixelsPerMeter,
    Pose, Rotation, ScaleDrawing, SiteProperties, NameOfSite,
};
use itertools::{Either, Itertools};
use optimization_engine::{panoc::*, *};
use std::collections::HashSet;

// Simple optimization purely based on measurement scale, used to transform between pixel and
// cartesian coordinates
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
                warn!("No measurements found on current drawing, skipping scaling");
            }
        }
    }
}

// The cost will be the sum of the square distances between pairs of points in constraints.
// Reference point pose is just world pose in meters, while the pose of the point to be optimized
// is expressed as a function of drawing translation, rotation and scale
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

// Calculates the partial derivatives for the cost function for each variable
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

fn align_drawing_pair(
    references: &HashSet<Entity>,
    target_drawing: Entity,
    constraints: &Vec<&Edge<Entity>>,
    params: &OptimizationParams,
    change: &mut OptimizationChangeParams,
) -> Option<(f64, f64, f64, f64)> {
    // Function that creates a pair of reference point and target point poses, their distance to be
    // minimized as part of the optimization
    let make_point_pair = |reference: Entity, target: Entity| {
        let reference_point = params
            .global_tfs
            .get(reference)
            .expect("Transform for anchor not found")
            .translation()
            .truncate()
            .to_array()
            .map(|t| t as f64);
        let target_point = params
            .anchors
            .get(target)
            .expect("Broken constraint anchor reference")
            .translation_for_category(Category::Drawing)
            .map(|t| t as f64);
        (reference_point, target_point)
    };
    // Guaranteed safe since caller passes a drawing entity
    let (_, _, target_pose, target_ppm) = params.drawings.get(target_drawing).unwrap();
    let mut matching_points = Vec::new();
    for edge in constraints.iter() {
        let start_parent = params
            .parents
            .get(edge.start())
            .expect("Anchor in constraint without drawing parent");
        let end_parent = params
            .parents
            .get(edge.end())
            .expect("Anchor in constraint without drawing parent");
        if (references.contains(&*start_parent)) & (target_drawing == **end_parent) {
            matching_points.push(make_point_pair(edge.start(), edge.end()));
        } else if (references.contains(&*end_parent)) & (target_drawing == **start_parent) {
            matching_points.push(make_point_pair(edge.end(), edge.start()));
        } else {
            continue;
        }
    }
    if matching_points.is_empty() {
        warn!(
            "No constraints found for drawing {:?}, skipping optimization",
            target_drawing
        );
        return None;
    }
    // Optimize the transform
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
    let x = target_pose.trans[0];
    let y = target_pose.trans[1];
    let theta = match target_pose.rot.as_yaw() {
        Rotation::Yaw(yaw) => yaw.radians(),
        _ => unreachable!(),
    };
    let s = target_ppm.0;
    let mut u = vec![x as f64, y as f64, theta as f64, s as f64];
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

    // Update transform parameters with results of the optimization
    let mut new_pose = target_pose.clone();
    new_pose.trans[0] = u[0] as f32;
    new_pose.trans[1] = u[1] as f32;
    new_pose.rot = Rotation::Yaw(Angle::Rad(u[2] as f32));
    change.pose.send(Change::new(new_pose, target_drawing));
    change
        .ppm
        .send(Change::new(PixelsPerMeter(u[3] as f32), target_drawing));
    Some((u[0], u[1], u[2], u[3]))
}

#[derive(SystemParam)]
pub struct OptimizationChangeParams<'w, 's> {
    pose: EventWriter<'w, 's, Change<Pose>>,
    ppm: EventWriter<'w, 's, Change<PixelsPerMeter>>,
}

#[derive(SystemParam)]
pub struct OptimizationParams<'w, 's> {
    drawings: Query<
        'w,
        's,
        (
            Entity,
            &'static Children,
            &'static Pose,
            &'static PixelsPerMeter,
        ),
        With<DrawingMarker>,
    >,
    global_tfs: Query<'w, 's, &'static GlobalTransform>,
    parents: Query<'w, 's, &'static Parent>,
    anchors: Query<'w, 's, &'static Anchor>,
    constraints: Query<'w, 's, &'static Edge<Entity>, With<ConstraintMarker>>,
}

pub fn align_level_drawings(
    levels: Query<&Children, With<LevelElevation>>,
    mut events: EventReader<AlignLevelDrawings>,
    params: OptimizationParams,
    mut change: OptimizationChangeParams,
) {
    // for e in events.iter() {
    //     // Get the matching points for this entity
    //     let level_children = levels
    //         .get(**e)
    //         .expect("Align level event sent to non level entity");
    //     let constraints = level_children
    //         .iter()
    //         .filter_map(|child| params.constraints.get(*child).ok())
    //         .collect::<Vec<_>>();
    //     if constraints.is_empty() {
    //         warn!("No constraints found for level, skipping optimization");
    //         continue;
    //     }
    //     let (references, layers): (HashSet<_>, Vec<_>) = level_children
    //         .iter()
    //         .filter_map(|child| params.drawings.get(*child).ok())
    //         .partition_map(|(e, _, _, _)| {
    //             if primary.0 == true {
    //                 Either::Left(e)
    //             } else {
    //                 Either::Right(e)
    //             }
    //         });
    //     if layers.is_empty() {
    //         warn!(
    //             "No non-primary drawings found for level, at least one drawing must be set to \
    //               non-primary to be optimized against primary drawings.Skipping optimization"
    //         );
    //         continue;
    //     }
    //     if references.is_empty() {
    //         warn!(
    //             "No primary drawings found for level. At least one drawing must be set to \
    //               primary to use as a reference for other drawings. Skipping optimization"
    //         );
    //         continue;
    //     }
    //     for layer_entity in layers {
    //         align_drawing_pair(
    //             &references,
    //             layer_entity,
    //             &constraints,
    //             &params,
    //             &mut change,
    //         );
    //     }
    // }
}

pub fn align_site_drawings(
    levels: Query<(Entity, &Children, &Parent), With<LevelElevation>>,
    sites: Query<&Children, With<NameOfSite>>,
    mut events: EventReader<AlignSiteDrawings>,
    params: OptimizationParams,
    mut change: OptimizationChangeParams,
) {
    // for e in events.iter() {
    //     // Get the levels that are children of the requested site
    //     let levels = levels
    //         .iter()
    //         .filter(|(_, _, p)| ***p == **e)
    //         .collect::<Vec<_>>();
    //     let reference_level = levels
    //         .iter()
    //         .min_by(|l_a, l_b| l_a.3.elevation.partial_cmp(&l_b.3.elevation).unwrap())
    //         .expect("Site has no levels");
    //     // Reference level will be the one with minimum elevation
    //     let references = reference_level
    //         .1
    //         .iter()
    //         .filter_map(|c| {
    //             params
    //                 .drawings
    //                 .get(*c)
    //                 .ok()
    //                 .filter(|(_, _, _, _, primary)| primary.0 == true)
    //         })
    //         .map(|(e, _, _, _, _)| e)
    //         .collect::<HashSet<_>>();
    //     // Layers to be optimized are primary drawings in the non reference level
    //     let layers = levels
    //         .iter()
    //         .filter_map(|(e, c, _)| (*e != reference_level.0).then(|| c.iter()))
    //         .flatten()
    //         .filter_map(|child| params.drawings.get(*child).ok())
    //         .filter_map(|(e, _, _, _, primary)| (primary.0 == true).then(|| e))
    //         .collect::<Vec<_>>();
    //     // Inter level constraints are children of the site
    //     let constraints = sites
    //         .get(**e)
    //         .expect("Align site sent to non site entity")
    //         .iter()
    //         .filter_map(|child| params.constraints.get(*child).ok())
    //         .collect::<Vec<_>>();
    //     if constraints.is_empty() {
    //         warn!("No constraints found for site, skipping optimization");
    //         continue;
    //     }
    //     if layers.is_empty() {
    //         warn!(
    //             "No other levels drawings found for site, at least one other level must have a \
    //               primary drawing to be optimized against reference level. Skipping optimization"
    //         );
    //         continue;
    //     }
    //     if references.is_empty() {
    //         warn!(
    //             "No reference level drawing found for site. At least one primary drawing must be \
    //               present in the lowest level to use as a reference for other levels. \
    //               Skipping optimization"
    //         );
    //         continue;
    //     }
    //     for layer_entity in layers {
    //         align_drawing_pair(
    //             &references,
    //             layer_entity,
    //             &constraints,
    //             &params,
    //             &mut change,
    //         );
    //     }
    // }
}
